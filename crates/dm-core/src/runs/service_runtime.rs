use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::runs::model::{LogSyncState, RunInstance, RunStatus, TerminationReason};
use crate::runs::runtime::RuntimeBackend;
use crate::runs::state::{
    apply_terminal_state, build_outcome, infer_failure_details, parse_failure_details,
    TerminalStateUpdate,
};
use crate::runs::{repo, runtime};

pub async fn stop_run(home: &Path, run_id: &str) -> Result<RunInstance> {
    let backend = runtime::default_backend();
    stop_run_with_backend(home, run_id, &backend).await
}

pub(super) async fn stop_run_with_backend<B: RuntimeBackend>(
    home: &Path,
    run_id: &str,
    backend: &B,
) -> Result<RunInstance> {
    let mut run = repo::load_run(home, run_id)?;
    let dora_uuid = run
        .dora_uuid
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Run '{}' has no dora UUID", run_id))?;

    match backend.stop(home, &dora_uuid).await {
        Ok(()) => {
            sync_run_outputs(home, &mut run)?;
            apply_terminal_state(
                &mut run,
                TerminalStateUpdate {
                    status: RunStatus::Stopped,
                    termination_reason: Some(TerminationReason::StoppedByUser),
                    exit_code: Some(0),
                    failure_reason: None,
                    failure_node: None,
                    failure_message: None,
                    observed_at: Some(Utc::now().to_rfc3339()),
                },
            );
            repo::save_run(home, &run)?;
            Ok(run)
        }
        Err(err) => {
            sync_run_outputs(home, &mut run)?;
            let (failure_node, failure_message) = parse_failure_details(&err.to_string());
            apply_terminal_state(
                &mut run,
                TerminalStateUpdate {
                    status: RunStatus::Failed,
                    termination_reason: Some(TerminationReason::NodeFailed),
                    exit_code: Some(1),
                    failure_reason: Some("node_failed".to_string()),
                    failure_node,
                    failure_message: Some(failure_message),
                    observed_at: Some(Utc::now().to_rfc3339()),
                },
            );
            repo::save_run(home, &run)?;
            Err(err)
        }
    }
}

pub fn refresh_run_statuses(home: &Path) -> Result<Vec<RunInstance>> {
    let mut runs = repo::list_run_instances(home)?;
    let backend = runtime::default_backend();
    refresh_run_statuses_with_backend(home, &mut runs, &backend)?;

    runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(runs)
}

pub(super) fn refresh_run_statuses_with_backend<B: RuntimeBackend>(
    home: &Path,
    runs: &mut [RunInstance],
    backend: &B,
) -> Result<()> {
    let runtime_items = match backend.list(home) {
        Ok(items) => items,
        Err(_) => return Ok(()),
    };
    let runtime_map: HashMap<String, RunStatus> = runtime_items
        .into_iter()
        .map(|item| (item.id, item.status))
        .collect();
    let now = Utc::now().to_rfc3339();

    for run in runs {
        if !run.status.is_running() {
            continue;
        }

        run.runtime_observed_at = Some(now.clone());
        let runtime_status = run
            .dora_uuid
            .as_ref()
            .and_then(|uuid| runtime_map.get(uuid))
            .copied();

        match runtime_status {
            Some(RunStatus::Running) => {
                run.outcome = build_outcome(RunStatus::Running, None, None, None);
                repo::save_run(home, run)?;
            }
            Some(RunStatus::Succeeded) => {
                sync_run_outputs(home, run)?;
                apply_terminal_state(
                    run,
                    TerminalStateUpdate {
                        status: RunStatus::Succeeded,
                        termination_reason: Some(TerminationReason::Completed),
                        exit_code: Some(0),
                        failure_reason: None,
                        failure_node: None,
                        failure_message: None,
                        observed_at: Some(now.clone()),
                    },
                );
                repo::save_run(home, run)?;
            }
            Some(RunStatus::Failed) => {
                sync_run_outputs(home, run)?;
                let (failure_node, failure_message) = infer_failure_details(home, run)
                    .unwrap_or_else(|| (run.failure_node.clone(), run.failure_message.clone()));
                apply_terminal_state(
                    run,
                    TerminalStateUpdate {
                        status: RunStatus::Failed,
                        termination_reason: Some(TerminationReason::NodeFailed),
                        exit_code: run.exit_code.or(Some(1)),
                        failure_reason: run
                            .failure_reason
                            .clone()
                            .or(Some("node_failed".to_string())),
                        failure_node,
                        failure_message,
                        observed_at: Some(now.clone()),
                    },
                );
                repo::save_run(home, run)?;
            }
            Some(RunStatus::Stopped) => {
                sync_run_outputs(home, run)?;
                apply_terminal_state(
                    run,
                    TerminalStateUpdate {
                        status: RunStatus::Stopped,
                        termination_reason: Some(TerminationReason::RuntimeStopped),
                        exit_code: run.exit_code.or(Some(0)),
                        failure_reason: None,
                        failure_node: None,
                        failure_message: None,
                        observed_at: Some(now.clone()),
                    },
                );
                repo::save_run(home, run)?;
            }
            None => {
                sync_run_outputs(home, run)?;
                apply_terminal_state(
                    run,
                    TerminalStateUpdate {
                        status: RunStatus::Stopped,
                        termination_reason: Some(TerminationReason::RuntimeLost),
                        exit_code: run.exit_code.or(Some(0)),
                        failure_reason: Some("runtime_lost".to_string()),
                        failure_node: None,
                        failure_message: Some(
                            "Dora runtime no longer reports this dataflow".to_string(),
                        ),
                        observed_at: Some(now.clone()),
                    },
                );
                repo::save_run(home, run)?;
            }
        }
    }
    Ok(())
}

pub fn sync_run_outputs(home: &Path, run: &mut RunInstance) -> Result<()> {
    let Some(dora_uuid) = run.dora_uuid.as_deref() else {
        return Ok(());
    };

    let source_dir = repo::run_out_dir(home, &run.run_id).join(dora_uuid);
    if !source_dir.exists() {
        run.log_sync.state = LogSyncState::Pending;
        return Ok(());
    }

    let logs_dir = repo::run_logs_dir(home, &run.run_id);
    fs::create_dir_all(&logs_dir)
        .with_context(|| format!("Failed to create logs dir {}", logs_dir.display()))?;

    let mut nodes = Vec::new();
    for entry in fs::read_dir(&source_dir)
        .with_context(|| format!("Failed to read Dora output dir {}", source_dir.display()))?
    {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();
        let Some(node_id) = filename
            .strip_prefix("log_")
            .and_then(|name| name.strip_suffix(".txt"))
        else {
            continue;
        };

        let target = logs_dir.join(format!("{}.log", node_id));
        fs::copy(entry.path(), &target).with_context(|| {
            format!(
                "Failed to copy node log {} to {}",
                entry.path().display(),
                target.display()
            )
        })?;
        nodes.push(node_id.to_string());
    }

    nodes.sort();
    nodes.dedup();
    run.node_count_observed = nodes.len() as u32;
    run.nodes_observed = nodes;
    run.log_sync.state = LogSyncState::Synced;
    run.log_sync.last_synced_at = Some(Utc::now().to_rfc3339());

    Ok(())
}
