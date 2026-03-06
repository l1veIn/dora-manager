use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::graph::{build_transpile_metadata, extract_node_ids_from_yaml};
use super::model::{
    LogSyncState, PaginatedRuns, RunDetail, RunInstance, RunListFilter, RunLogChunk, RunLogSync,
    RunSource, RunStatus, RunSummary, StartConflictStrategy, StartRunResult, TerminationReason,
};
use super::state::{
    apply_terminal_state, build_outcome, infer_failure_details, parse_failure_details,
};
use super::{repo, runtime};
use crate::runs::runtime::RuntimeBackend;

pub fn list_runs(home: &Path, limit: i64, offset: i64) -> Result<PaginatedRuns> {
    list_runs_filtered(home, limit, offset, &RunListFilter::default())
}

pub fn list_runs_filtered(
    home: &Path,
    limit: i64,
    offset: i64,
    filter: &RunListFilter,
) -> Result<PaginatedRuns> {
    let runs = refresh_run_statuses(home)?;
    let runs = apply_run_list_filter(runs, filter);
    let total = runs.len() as i64;
    let offset = offset.max(0) as usize;
    let limit = limit.max(1) as usize;

    Ok(PaginatedRuns {
        runs: runs
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(to_summary)
            .collect(),
        total,
        limit: limit as i64,
        offset: offset as i64,
    })
}

pub fn get_active_run(home: &Path) -> Result<Option<RunInstance>> {
    Ok(list_active_runs(home)?.into_iter().next())
}

pub fn list_active_runs(home: &Path) -> Result<Vec<RunInstance>> {
    Ok(refresh_run_statuses(home)?
        .into_iter()
        .filter(|run| run.status.is_running())
        .collect())
}

pub async fn start_run_from_yaml(
    home: &Path,
    yaml: &str,
    dataflow_name: &str,
) -> Result<StartRunResult> {
    start_run_from_yaml_with_source_and_strategy(
        home,
        yaml,
        dataflow_name,
        RunSource::Unknown,
        StartConflictStrategy::Fail,
    )
    .await
}

pub async fn start_run_from_yaml_with_strategy(
    home: &Path,
    yaml: &str,
    dataflow_name: &str,
    strategy: StartConflictStrategy,
) -> Result<StartRunResult> {
    start_run_from_yaml_with_source_and_strategy(
        home,
        yaml,
        dataflow_name,
        RunSource::Unknown,
        strategy,
    )
    .await
}

pub async fn start_run_from_yaml_with_source_and_strategy(
    home: &Path,
    yaml: &str,
    dataflow_name: &str,
    source: RunSource,
    strategy: StartConflictStrategy,
) -> Result<StartRunResult> {
    if let Some(active) = find_active_run_by_name(home, dataflow_name)? {
        match strategy {
            StartConflictStrategy::Fail => bail!(
                "Dataflow '{}' is already running as run {}. Stop it first or retry with force.",
                dataflow_name,
                active.run_id
            ),
            StartConflictStrategy::StopAndRestart => {
                stop_run(home, &active.run_id).await?;
            }
        }
    }

    let run_id = Uuid::new_v4().to_string();
    repo::create_layout(home, &run_id)?;

    let snapshot_path = repo::run_snapshot_path(home, &run_id);
    fs::write(&snapshot_path, yaml)
        .with_context(|| format!("Failed to write snapshot {}", snapshot_path.display()))?;

    let dataflow_hash = format!("sha256:{:x}", Sha256::digest(yaml.as_bytes()));
    let transpiled = crate::dataflow::transpile_graph_for_run(home, &snapshot_path, &run_id)
        .with_context(|| format!("Failed to transpile '{}'", dataflow_name))?;
    let transpiled_path = repo::run_transpiled_path(home, &run_id);
    fs::write(&transpiled_path, serde_yaml::to_string(&transpiled)?).with_context(|| {
        format!(
            "Failed to write transpiled graph {}",
            transpiled_path.display()
        )
    })?;

    let nodes_expected = extract_node_ids_from_yaml(yaml)?;
    let transpile = build_transpile_metadata(&transpiled);
    let has_panel = !transpile.panel_node_ids.is_empty();
    let mut run = RunInstance {
        schema_version: 1,
        run_id: run_id.clone(),
        dora_uuid: None,
        dataflow_name: dataflow_name.to_string(),
        dataflow_hash,
        source,
        has_panel,
        status: RunStatus::Running,
        termination_reason: None,
        failure_reason: None,
        failure_node: None,
        failure_message: None,
        started_at: Utc::now().to_rfc3339(),
        stopped_at: None,
        runtime_observed_at: None,
        exit_code: None,
        outcome: build_outcome(RunStatus::Running, None, None, None),
        transpile,
        log_sync: RunLogSync::default(),
        node_count_expected: nodes_expected.len() as u32,
        node_count_observed: 0,
        nodes_expected,
        nodes_observed: Vec::new(),
    };
    repo::save_run(home, &run)?;

    let backend = runtime::default_backend();
    match backend.start_detached(home, &transpiled_path).await {
        Ok((dora_uuid, message)) => {
            run.dora_uuid = dora_uuid;
            repo::save_run(home, &run)?;
            Ok(StartRunResult { run, message })
        }
        Err(err) => {
            apply_terminal_state(
                &mut run,
                RunStatus::Failed,
                Some(TerminationReason::StartFailed),
                None,
                Some("start_failed".to_string()),
                None,
                Some(err.to_string()),
                Some(Utc::now().to_rfc3339()),
            );
            repo::save_run(home, &run)?;
            Err(err)
        }
    }
}

pub async fn start_run_from_file(home: &Path, file_path: &Path) -> Result<StartRunResult> {
    start_run_from_file_with_source_and_strategy(
        home,
        file_path,
        RunSource::Unknown,
        StartConflictStrategy::Fail,
    )
    .await
}

pub async fn start_run_from_file_with_strategy(
    home: &Path,
    file_path: &Path,
    strategy: StartConflictStrategy,
) -> Result<StartRunResult> {
    start_run_from_file_with_source_and_strategy(home, file_path, RunSource::Unknown, strategy)
        .await
}

pub async fn start_run_from_file_with_source_and_strategy(
    home: &Path,
    file_path: &Path,
    source: RunSource,
    strategy: StartConflictStrategy,
) -> Result<StartRunResult> {
    let yaml = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read graph file '{}'", file_path.display()))?;
    let dataflow_name = file_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    start_run_from_yaml_with_source_and_strategy(home, &yaml, &dataflow_name, source, strategy)
        .await
}

pub async fn stop_run(home: &Path, run_id: &str) -> Result<RunInstance> {
    let mut run = repo::load_run(home, run_id)?;
    let dora_uuid = run
        .dora_uuid
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Run '{}' has no dora UUID", run_id))?;

    let backend = runtime::default_backend();
    match backend.stop(home, &dora_uuid).await {
        Ok(()) => {
            sync_run_outputs(home, &mut run)?;
            apply_terminal_state(
                &mut run,
                RunStatus::Stopped,
                Some(TerminationReason::StoppedByUser),
                Some(0),
                None,
                None,
                None,
                Some(Utc::now().to_rfc3339()),
            );
            repo::save_run(home, &run)?;
            Ok(run)
        }
        Err(err) => {
            sync_run_outputs(home, &mut run)?;
            let (failure_node, failure_message) = parse_failure_details(&err.to_string());
            apply_terminal_state(
                &mut run,
                RunStatus::Failed,
                Some(TerminationReason::NodeFailed),
                Some(1),
                Some("node_failed".to_string()),
                failure_node,
                Some(failure_message),
                Some(Utc::now().to_rfc3339()),
            );
            repo::save_run(home, &run)?;
            Err(err)
        }
    }
}

pub fn get_run(home: &Path, run_id: &str) -> Result<RunDetail> {
    let _ = refresh_run_statuses(home)?;
    let mut run = repo::load_run(home, run_id)?;
    sync_run_outputs(home, &mut run)?;
    repo::save_run(home, &run)?;

    Ok(RunDetail {
        summary: to_summary(run.clone()),
        nodes: repo::list_run_nodes(home, &run.run_id)?,
    })
}

pub fn read_run_log(home: &Path, run_id: &str, node_id: &str) -> Result<String> {
    let path = repo::run_logs_dir(home, run_id).join(format!("{}.log", node_id));
    if !path.exists() {
        let mut run = repo::load_run(home, run_id)?;
        sync_run_outputs(home, &mut run)?;
        repo::save_run(home, &run)?;
    }
    repo::read_run_log_file(home, run_id, node_id)
}

pub fn read_run_transpiled(home: &Path, run_id: &str) -> Result<String> {
    repo::read_run_transpiled(home, run_id)
}

pub fn read_run_log_chunk(
    home: &Path,
    run_id: &str,
    node_id: &str,
    offset: u64,
) -> Result<RunLogChunk> {
    let _ = refresh_run_statuses(home)?;
    let mut run = repo::load_run(home, run_id)?;
    sync_run_outputs(home, &mut run)?;
    repo::save_run(home, &run)?;

    let log_path = repo::run_logs_dir(home, run_id).join(format!("{}.log", node_id));
    let (content, next_offset) = if log_path.exists() {
        repo::read_run_log_chunk(home, run_id, node_id, offset)?
    } else {
        (String::new(), 0)
    };

    Ok(RunLogChunk {
        run_id: run_id.to_string(),
        node_id: node_id.to_string(),
        offset,
        next_offset,
        content,
        finished: !run.status.is_running(),
        status: run.status.as_str().to_string(),
    })
}

pub fn delete_run(home: &Path, run_id: &str) -> Result<()> {
    repo::delete_run(home, run_id)?;
    let store = crate::events::EventStore::open(home)?;
    let _ = store.delete_by_case_id(run_id);
    Ok(())
}

pub fn clean_runs(home: &Path, keep: usize) -> Result<u32> {
    let all = list_runs(home, 10000, 0)?;
    let mut deleted = 0u32;
    if all.runs.len() > keep {
        for run in &all.runs[keep..] {
            if let Err(e) = delete_run(home, &run.id) {
                eprintln!("Warning: failed to clean run {}: {}", run.id, e);
            } else {
                deleted += 1;
            }
        }
    }
    Ok(deleted)
}

pub fn refresh_run_statuses(home: &Path) -> Result<Vec<RunInstance>> {
    let mut runs = repo::list_run_instances(home)?;
    let backend = runtime::default_backend();
    let runtime_map: HashMap<String, RunStatus> = backend
        .list(home)
        .unwrap_or_default()
        .into_iter()
        .map(|item| (item.id, item.status))
        .collect();
    let now = Utc::now().to_rfc3339();

    for run in &mut runs {
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
                    RunStatus::Succeeded,
                    Some(TerminationReason::Completed),
                    Some(0),
                    None,
                    None,
                    None,
                    Some(now.clone()),
                );
                repo::save_run(home, run)?;
            }
            Some(RunStatus::Failed) => {
                sync_run_outputs(home, run)?;
                let (failure_node, failure_message) = infer_failure_details(home, run)
                    .unwrap_or_else(|| (run.failure_node.clone(), run.failure_message.clone()));
                apply_terminal_state(
                    run,
                    RunStatus::Failed,
                    Some(TerminationReason::NodeFailed),
                    run.exit_code.or(Some(1)),
                    run.failure_reason
                        .clone()
                        .or(Some("node_failed".to_string())),
                    failure_node,
                    failure_message,
                    Some(now.clone()),
                );
                repo::save_run(home, run)?;
            }
            Some(RunStatus::Stopped) => {
                sync_run_outputs(home, run)?;
                apply_terminal_state(
                    run,
                    RunStatus::Stopped,
                    Some(TerminationReason::RuntimeStopped),
                    run.exit_code.or(Some(0)),
                    None,
                    None,
                    None,
                    Some(now.clone()),
                );
                repo::save_run(home, run)?;
            }
            None => {
                sync_run_outputs(home, run)?;
                apply_terminal_state(
                    run,
                    RunStatus::Stopped,
                    Some(TerminationReason::RuntimeLost),
                    run.exit_code.or(Some(0)),
                    Some("runtime_lost".to_string()),
                    None,
                    Some("Dora runtime no longer reports this dataflow".to_string()),
                    Some(now.clone()),
                );
                repo::save_run(home, run)?;
            }
        }
    }

    runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(runs)
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

fn to_summary(run: RunInstance) -> RunSummary {
    RunSummary {
        id: run.run_id,
        name: run.dataflow_name,
        started_at: run.started_at,
        finished_at: run.stopped_at,
        exit_code: run.exit_code,
        source: run.source.as_str().to_string(),
        has_panel: run.has_panel,
        node_count: run.node_count_observed,
        status: run.status.as_str().to_string(),
        termination_reason: run
            .termination_reason
            .map(|reason| reason.as_str().to_string()),
        outcome_summary: run.outcome.summary,
        dora_uuid: run.dora_uuid,
    }
}

fn find_active_run_by_name(home: &Path, dataflow_name: &str) -> Result<Option<RunInstance>> {
    Ok(refresh_run_statuses(home)?
        .into_iter()
        .find(|run| run.status.is_running() && run.dataflow_name == dataflow_name))
}

fn apply_run_list_filter(runs: Vec<RunInstance>, filter: &RunListFilter) -> Vec<RunInstance> {
    let normalized_status = filter
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());
    let normalized_search = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());

    runs.into_iter()
        .filter(|run| {
            if let Some(has_panel) = filter.has_panel {
                if run.has_panel != has_panel {
                    return false;
                }
            }

            if let Some(status) = normalized_status.as_deref() {
                if run.status.as_str() != status {
                    return false;
                }
            }

            if let Some(search) = normalized_search.as_deref() {
                let run_id = run.run_id.to_ascii_lowercase();
                let dataflow_name = run.dataflow_name.to_ascii_lowercase();
                if !run_id.contains(search) && !dataflow_name.contains(search) {
                    return false;
                }
            }

            true
        })
        .collect()
}
