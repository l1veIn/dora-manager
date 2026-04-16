use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::runs::graph::{build_transpile_metadata, extract_node_ids_from_yaml};
use crate::runs::model::{
    RunInstance, RunLogSync, RunSource, RunStatus, StartConflictStrategy, StartRunResult,
    TerminationReason,
};
use crate::runs::runtime::RuntimeBackend;
use crate::runs::state::{apply_terminal_state, build_outcome, TerminalStateUpdate};
use crate::runs::{repo, runtime};

pub async fn start_run_from_yaml(
    home: &Path,
    yaml: &str,
    dataflow_name: &str,
) -> Result<StartRunResult> {
    start_run_from_yaml_with_source_and_strategy(
        home,
        yaml,
        dataflow_name,
        None,
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
        None,
        RunSource::Unknown,
        strategy,
    )
    .await
}

pub async fn start_run_from_yaml_with_source_and_strategy(
    home: &Path,
    yaml: &str,
    dataflow_name: &str,
    view_json: Option<&str>,
    source: RunSource,
    strategy: StartConflictStrategy,
) -> Result<StartRunResult> {
    let backend = runtime::default_backend();
    start_run_from_yaml_with_source_and_strategy_and_backend(
        home,
        yaml,
        dataflow_name,
        view_json,
        source,
        strategy,
        &backend,
    )
    .await
}

pub(super) async fn start_run_from_yaml_with_source_and_strategy_and_backend<B: RuntimeBackend>(
    home: &Path,
    yaml: &str,
    dataflow_name: &str,
    view_json: Option<&str>,
    source: RunSource,
    strategy: StartConflictStrategy,
    backend: &B,
) -> Result<StartRunResult> {
    let mut executable = crate::dataflow::inspect_yaml(home, yaml);
    
    // Auto-install missing nodes if we have git URLs
    if !executable.summary.missing_nodes.is_empty() {
        if let Some(missing_with_git) = &executable.summary.missing_nodes_with_git_url {
            let mut installed_any = false;
            for (node_id, git_url) in missing_with_git {
                // Import the node from git
                match crate::node::import_git(home, node_id, git_url).await {
                    Ok(_) => {
                        // Install the node
                        match crate::node::install_node(home, node_id).await {
                            Ok(_) => {
                                installed_any = true;
                            }
                            Err(e) => {
                                // Log the error but continue with other nodes
                                eprintln!("Warning: Failed to install node '{}': {}", node_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        // Log the error but continue with other nodes
                        eprintln!("Warning: Failed to import node '{}': {}", node_id, e);
                    }
                }
            }
            
            // Re-inspect the YAML if we installed any nodes
            if installed_any {
                executable = crate::dataflow::inspect_yaml(home, yaml);
            }
        }
    }
    
    if !executable.summary.can_run {
        if executable.summary.invalid_yaml {
            bail!(
                "Dataflow '{}' is not executable: invalid yaml{}",
                dataflow_name,
                executable
                    .summary
                    .error
                    .as_deref()
                    .map(|err| format!(": {}", err))
                    .unwrap_or_default()
            );
        }
        if !executable.summary.missing_nodes.is_empty() {
            bail!(
                "Dataflow '{}' is not executable: missing nodes: {}",
                dataflow_name,
                executable.summary.missing_nodes.join(", ")
            );
        }
        bail!("Dataflow '{}' is not executable", dataflow_name);
    }

    if let Some(active) = super::find_active_run_by_name_with_backend(home, dataflow_name, backend)?
    {
        match strategy {
            StartConflictStrategy::Fail => bail!(
                "Dataflow '{}' is already running as run {}. Stop it first or retry with force.",
                dataflow_name,
                active.run_id
            ),
            StartConflictStrategy::StopAndRestart => {
                super::service_runtime::stop_run_with_backend(home, &active.run_id, backend)
                    .await?;
            }
        }
    }

    let run_id = Uuid::new_v4().to_string();
    repo::create_layout(home, &run_id)?;

    let snapshot_path = repo::run_snapshot_path(home, &run_id);
    fs::write(&snapshot_path, yaml)
        .with_context(|| format!("Failed to write snapshot {}", snapshot_path.display()))?;

    if let Some(vj) = view_json {
        let view_json_path = repo::run_view_json_path(home, &run_id);
        fs::write(&view_json_path, vj)
            .with_context(|| format!("Failed to write view.json {}", view_json_path.display()))?;
    }

    let dataflow_hash = format!("sha256:{:x}", Sha256::digest(yaml.as_bytes()));
    let transpile_result = crate::dataflow::transpile_graph_for_run(home, &snapshot_path, &run_id)
        .with_context(|| format!("Failed to transpile '{}'", dataflow_name))?;
    let transpiled_path = repo::run_transpiled_path(home, &run_id);
    fs::write(
        &transpiled_path,
        serde_yaml::to_string(&transpile_result.yaml)?,
    )
    .with_context(|| {
        format!(
            "Failed to write transpiled graph {}",
            transpiled_path.display()
        )
    })?;

    let nodes_expected = extract_node_ids_from_yaml(yaml)?;
    let transpile = build_transpile_metadata(&transpile_result.yaml);
    let mut run = RunInstance {
        schema_version: 1,
        run_id: run_id.clone(),
        dora_uuid: None,
        dataflow_name: dataflow_name.to_string(),
        dataflow_hash,
        source,
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

    match backend.start_detached(home, &transpiled_path).await {
        Ok((Some(dora_uuid), message)) => {
            run.dora_uuid = Some(dora_uuid);
            repo::save_run(home, &run)?;
            Ok(StartRunResult { run, message })
        }
        Ok((None, message)) => {
            let err = anyhow::anyhow!(
                "Dora started '{}' but did not return a runtime UUID. Output: {}",
                dataflow_name,
                message
            );
            apply_terminal_state(
                &mut run,
                TerminalStateUpdate {
                    status: RunStatus::Failed,
                    termination_reason: Some(TerminationReason::StartFailed),
                    exit_code: None,
                    failure_reason: Some("start_failed".to_string()),
                    failure_node: None,
                    failure_message: Some(err.to_string()),
                    observed_at: Some(Utc::now().to_rfc3339()),
                },
            );
            repo::save_run(home, &run)?;
            Err(err)
        }
        Err(err) => {
            apply_terminal_state(
                &mut run,
                TerminalStateUpdate {
                    status: RunStatus::Failed,
                    termination_reason: Some(TerminationReason::StartFailed),
                    exit_code: None,
                    failure_reason: Some("start_failed".to_string()),
                    failure_node: None,
                    failure_message: Some(err.to_string()),
                    observed_at: Some(Utc::now().to_rfc3339()),
                },
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
        None,
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
    start_run_from_file_with_source_and_strategy(
        home,
        file_path,
        None,
        RunSource::Unknown,
        strategy,
    )
    .await
}

pub async fn start_run_from_file_with_source_and_strategy(
    home: &Path,
    file_path: &Path,
    view_json: Option<&str>,
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
    start_run_from_yaml_with_source_and_strategy(
        home,
        &yaml,
        &dataflow_name,
        view_json,
        source,
        strategy,
    )
    .await
}
