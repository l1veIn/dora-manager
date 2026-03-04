use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

use crate::config;

/// Resolve the path to the active dora binary managed by dm.
pub fn active_dora_bin(home: &Path) -> Result<PathBuf> {
    let cfg = config::load_config(home)?;
    let version = cfg
        .active_version
        .ok_or_else(|| anyhow::anyhow!("No active dora version. Run `dm install` first."))?;
    let bin = config::versions_dir(home).join(&version).join("dora");
    if !bin.exists() {
        anyhow::bail!(
            "dora binary not found at {}. Run `dm install {}` to fix.",
            bin.display(),
            version
        );
    }
    Ok(bin)
}

/// Run a dora subcommand using the active managed binary.
/// Returns (exit_code, stdout, stderr).
pub async fn run_dora(
    home: &Path,
    args: &[String],
    verbose: bool,
) -> Result<(i32, String, String)> {
    let bin = active_dora_bin(home)?;
    if verbose {
        eprintln!("[dm] exec: {} {}", bin.display(), args.join(" "));
    }
    let output = Command::new(&bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to run dora at {}", bin.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    Ok((code, stdout, stderr))
}

/// Run dora with inherited stdio (for interactive / pass-through commands).
pub async fn exec_dora(home: &Path, args: &[String], verbose: bool) -> Result<i32> {
    let bin = active_dora_bin(home)?;
    if verbose {
        eprintln!("[dm] exec: {} {}", bin.display(), args.join(" "));
    }
    let status = Command::new(&bin)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("Failed to run dora at {}", bin.display()))?;

    Ok(status.code().unwrap_or(-1))
}

/// Run dora with real-time forwarded stdio, capturing the dataflow UUID.
///
/// Both stdout and stderr are piped and forwarded line-by-line so we can
/// extract the dataflow ID from dora's output (it goes to stderr).
/// Stdin is inherited for interactive use.
/// Returns (exit_code, Option<dataflow_id>).
pub async fn exec_dora_capture_id(
    home: &Path,
    args: &[String],
    verbose: bool,
) -> Result<(i32, Option<String>)> {
    let bin = active_dora_bin(home)?;
    if verbose {
        eprintln!("[dm] exec: {} {}", bin.display(), args.join(" "));
    }
    let mut child = Command::new(&bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to spawn dora at {}", bin.display()))?;

    let stdout = child.stdout.take().expect("stdout should be piped");
    let stderr = child.stderr.take().expect("stderr should be piped");

    let dataflow_id = std::sync::Arc::new(tokio::sync::Mutex::new(None::<String>));

    // Forward stdout
    let stdout_handle = {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                println!("{}", line);
            }
        })
    };

    // Forward stderr and extract dataflow ID
    let stderr_handle = {
        let df_id = dataflow_id.clone();
        let mut reader = tokio::io::BufReader::new(stderr).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                eprintln!("{}", line);
                let mut guard = df_id.lock().await;
                if guard.is_none() {
                    if let Some(uuid) = line
                        .strip_prefix("dataflow start triggered: ")
                        .or_else(|| line.strip_prefix("dataflow started: "))
                    {
                        *guard = Some(uuid.trim().to_string());
                    }
                }
            }
        })
    };

    let _ = tokio::join!(stdout_handle, stderr_handle);
    let status = child.wait().await?;
    let result_id = dataflow_id.lock().await.clone();
    Ok((status.code().unwrap_or(-1), result_id))
}

/// Get the version string from a dora binary.
pub async fn get_dora_version(bin_path: &Path) -> Result<String> {
    let output = Command::new(bin_path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    // Output is typically "dora-cli 0.4.1\ndora-message: 0.7.0\n..." — take first line
    let first_line = out.lines().next().unwrap_or("").trim();
    Ok(first_line
        .split_whitespace()
        .last()
        .unwrap_or("unknown")
        .to_string())
}
