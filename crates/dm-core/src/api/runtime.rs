use std::path::Path;

use anyhow::{Context, Result};

use crate::events::{EventSource, OperationEvent};
use crate::runs::RunInstance;
use crate::{config, dora, types::*};

/// Get runtime status overview
pub async fn status(home: &Path, verbose: bool) -> Result<StatusReport> {
    let cfg = config::load_config(home)?;
    let dm_home = home.display().to_string();

    if cfg.active_version.is_none() {
        return Ok(StatusReport {
            active_version: None,
            actual_version: None,
            dm_home,
            runtime_running: false,
            runtime_output: String::new(),
            active_runs: Vec::new(),
            recent_runs: Vec::new(),
            dora_probe: Vec::new(),
        });
    }

    let ver = cfg.active_version.as_ref().unwrap().clone();
    let bin = config::versions_dir(home).join(&ver);
    let dora_bin = bin.join("dora");

    let check_args = vec!["check".to_string()];
    let list_args = vec!["list".to_string()];
    let (version_result, check_result, list_result) = tokio::join!(
        dora::get_dora_version(&dora_bin),
        dora::run_dora(home, &check_args, verbose),
        dora::run_dora(home, &list_args, verbose),
    );

    let actual_version = version_result.ok();

    let (runtime_running, runtime_output) = match check_result {
        Ok((code, stdout, stderr)) => (
            code == 0,
            if code == 0 {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            },
        ),
        _ => (false, String::new()),
    };

    let runs = crate::runs::refresh_run_statuses(home).unwrap_or_default();
    let active_runs = runs
        .iter()
        .filter(|run| run.status.is_running())
        .cloned()
        .map(to_status_run_entry)
        .collect();
    let recent_runs = runs
        .iter()
        .filter(|run| !run.status.is_running())
        .take(3)
        .cloned()
        .map(to_status_run_entry)
        .collect();
    let dora_probe = if verbose {
        match list_result {
            Ok((0, stdout, _)) => build_dora_probe(&stdout, &runs),
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    Ok(StatusReport {
        active_version: Some(ver),
        actual_version,
        dm_home,
        runtime_running,
        runtime_output,
        active_runs,
        recent_runs,
        dora_probe,
    })
}

fn build_dora_probe(stdout: &str, runs: &[RunInstance]) -> Vec<RuntimeDataflowStatus> {
    let runtime_infos = dora::parse_runtime_infos(stdout);

    runtime_infos
        .into_iter()
        .map(|item| {
            let run = runs
                .iter()
                .find(|run| run.dora_uuid.as_deref() == Some(item.id.as_str()));

            RuntimeDataflowStatus {
                id: item.id.clone(),
                dataflow_name: run
                    .map(|run| run.dataflow_name.clone())
                    .or_else(|| item.name.clone())
                    .unwrap_or_else(|| item.id.clone()),
                runtime_name: item.name,
                status: item.status.unwrap_or_else(|| "Unknown".to_string()),
                expected_nodes: run.map(|run| run.node_count_expected).unwrap_or(0),
                observed_nodes: run
                    .map(|run| run.node_count_observed)
                    .or(item.nodes)
                    .unwrap_or(0),
                cpu: item.cpu,
                memory: item.memory,
            }
        })
        .collect()
}

fn to_status_run_entry(run: RunInstance) -> StatusRunEntry {
    StatusRunEntry {
        run_id: run.run_id,
        dataflow_name: run.dataflow_name,
        status: run.status.as_str().to_string(),
        started_at: run.started_at,
        finished_at: run.stopped_at,
        expected_nodes: run.node_count_expected,
        observed_nodes: run.node_count_observed,
        has_panel: run.has_panel,
        dora_uuid: run.dora_uuid,
        outcome_summary: run.outcome.summary,
    }
}

/// Start dora coordinator + daemon
pub async fn up(home: &Path, verbose: bool) -> Result<RuntimeResult> {
    let op = OperationEvent::new(home, EventSource::Core, "runtime.up");
    op.emit_start();

    let result = async {
        let bin = dora::active_dora_bin(home)?;
        if verbose {
            eprintln!("[dm] exec: {} up", bin.display());
        }

        let mut child = tokio::process::Command::new(&bin)
            .arg("up")
            .current_dir(home)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn dora at {}", bin.display()))?;

        for i in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            if let Some(exit) = child.try_wait()? {
                if !exit.success() {
                    let stderr = if let Some(mut se) = child.stderr.take() {
                        use tokio::io::AsyncReadExt;
                        let mut buf = String::new();
                        se.read_to_string(&mut buf).await.ok();
                        buf
                    } else {
                        String::new()
                    };
                    return Ok(RuntimeResult {
                        success: false,
                        message: stderr.trim().to_string(),
                    });
                }
            }

            if is_runtime_running(home, verbose).await {
                return Ok(RuntimeResult {
                    success: true,
                    message: "Dora runtime started successfully.".to_string(),
                });
            }
            if verbose {
                eprintln!(
                    "[dm] Waiting for runtime to initialize... (attempt {}/10)",
                    i + 1
                );
            }
        }

        Ok(RuntimeResult {
            success: false,
            message: "Timed out waiting for dora runtime to start.".to_string(),
        })
    }
    .await;

    op.emit_result(&result);
    result
}

/// Stop dora coordinator + daemon
pub async fn down(home: &Path, verbose: bool) -> Result<RuntimeResult> {
    let op = OperationEvent::new(home, EventSource::Core, "runtime.down");
    op.emit_start();

    let result = async {
        let (code, stdout, stderr) =
            dora::run_dora(home, &["destroy".to_string()], verbose).await?;
        if code != 0 {
            return Ok(RuntimeResult {
                success: false,
                message: stderr.trim().to_string(),
            });
        }

        for i in 0..3 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if !is_runtime_running(home, verbose).await {
                return Ok(RuntimeResult {
                    success: true,
                    message: stdout.trim().to_string(),
                });
            }
            if verbose {
                eprintln!(
                    "[dm] Waiting for runtime to shut down... (attempt {}/3)",
                    i + 1
                );
            }
        }

        Ok(RuntimeResult {
            success: false,
            message: "dora destroy returned success but runtime is still running.".to_string(),
        })
    }
    .await;

    op.emit_result(&result);
    result
}

/// Check if dora runtime (coordinator + daemon) is currently running.
pub async fn is_runtime_running(home: &Path, verbose: bool) -> bool {
    if let Ok((code, _, _)) = dora::run_dora(home, &["check".to_string()], verbose).await {
        code == 0
    } else {
        false
    }
}

/// Ensure dora runtime is running; auto-start if not.
pub async fn ensure_runtime_up(home: &Path, verbose: bool) -> Result<()> {
    if !is_runtime_running(home, verbose).await {
        let result = up(home, verbose).await?;
        if !result.success {
            anyhow::bail!("Failed to start dora runtime: {}", result.message);
        }
    }
    Ok(())
}

/// If no active runs remain, silently shut down the runtime.
/// Refreshes run statuses first to detect naturally finished dataflows.
pub async fn auto_down_if_idle(home: &Path, verbose: bool) {
    if !is_runtime_running(home, verbose).await {
        return;
    }
    let _ = crate::runs::refresh_run_statuses(home);
    if let Ok(active) = crate::runs::list_active_runs(home) {
        if active.is_empty() {
            let _ = down(home, verbose).await;
        }
    }
}

/// Pass-through: execute any dora CLI command interactively
pub async fn passthrough(home: &Path, args: &[String], verbose: bool) -> Result<i32> {
    let op = OperationEvent::new(home, EventSource::Core, "passthrough").attr("args", args);
    op.emit_start();

    let result = dora::exec_dora(home, args, verbose).await;
    op.emit_result(&result);
    result
}
