use std::path::Path;

use anyhow::{Context, Result};

use crate::events::{EventSource, OperationEvent};
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
            dataflows: Vec::new(),
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

    let dataflows = match list_result {
        Ok((0, stdout, _)) => {
            let output = stdout.trim();
            if !output.is_empty() && !output.contains("No running dataflow") {
                output.lines().map(|l| l.to_string()).collect()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };

    Ok(StatusReport {
        active_version: Some(ver),
        actual_version,
        dm_home,
        runtime_running,
        runtime_output,
        dataflows,
    })
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
                eprintln!("[dm] Waiting for runtime to initialize... (attempt {}/10)", i + 1);
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
        let (code, stdout, stderr) = dora::run_dora(home, &["destroy".to_string()], verbose).await?;
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
                eprintln!("[dm] Waiting for runtime to shut down... (attempt {}/3)", i + 1);
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

/// Pass-through: execute any dora CLI command interactively
pub async fn passthrough(home: &Path, args: &[String], verbose: bool) -> Result<i32> {
    let op = OperationEvent::new(home, EventSource::Core, "passthrough").attr("args", args);
    op.emit_start();

    let result = dora::exec_dora(home, args, verbose).await;
    op.emit_result(&result);
    result
}
