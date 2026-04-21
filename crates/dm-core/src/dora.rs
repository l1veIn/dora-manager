use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::config;

#[derive(Debug, Clone)]
pub struct DataflowRuntimeInfo {
    pub id: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub nodes: Option<u32>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

/// Resolve the path to the active dora binary managed by dm.
pub fn active_dora_bin(home: &Path) -> Result<PathBuf> {
    let cfg = config::load_config(home)?;
    let version = cfg
        .active_version
        .ok_or_else(|| anyhow::anyhow!("No active dora version. Run `dm install` first."))?;
    let bin = config::dora_bin_path(&config::versions_dir(home).join(&version));
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

pub async fn list_dataflow_ids(home: &Path, verbose: bool) -> Result<Vec<String>> {
    let (code, stdout, stderr) = run_dora(home, &["list".to_string()], verbose).await?;
    if code != 0 {
        anyhow::bail!(stderr.trim().to_string());
    }

    Ok(parse_runtime_infos(&stdout)
        .into_iter()
        .map(|item| item.id)
        .collect())
}

pub async fn list_dataflows(home: &Path, verbose: bool) -> Result<Vec<DataflowRuntimeInfo>> {
    let (code, stdout, stderr) = run_dora(home, &["list".to_string()], verbose).await?;
    if code != 0 {
        anyhow::bail!(stderr.trim().to_string());
    }

    Ok(parse_runtime_infos(&stdout))
}

pub fn list_dataflow_ids_blocking(home: &Path, verbose: bool) -> Result<Vec<String>> {
    Ok(list_dataflows_blocking(home, verbose)?
        .into_iter()
        .map(|item| item.id)
        .collect())
}

pub fn list_dataflows_blocking(home: &Path, verbose: bool) -> Result<Vec<DataflowRuntimeInfo>> {
    let bin = active_dora_bin(home)?;
    if verbose {
        eprintln!("[dm] exec: {} list", bin.display());
    }

    let output = StdCommand::new(&bin)
        .arg("list")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to run dora at {}", bin.display()))?;

    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(parse_runtime_infos(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

pub fn check_runtime_blocking(home: &Path, verbose: bool) -> Result<(bool, String)> {
    let bin = active_dora_bin(home)?;
    if verbose {
        eprintln!("[dm] exec: {} check", bin.display());
    }

    let output = StdCommand::new(&bin)
        .arg("check")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to run dora at {}", bin.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Ok((
        output.status.success(),
        if output.status.success() {
            stdout
        } else {
            stderr
        },
    ))
}

pub(crate) fn parse_runtime_infos(stdout: &str) -> Vec<DataflowRuntimeInfo> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("UUID"))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let id = parts.first()?.to_string();
            if uuid::Uuid::parse_str(&id).is_err() {
                return None;
            }
            let name = parts.get(1).map(|value| value.to_string());
            let status = parts.get(2).map(|value| value.to_string());
            let nodes = parts.get(3).and_then(|value| value.parse::<u32>().ok());
            let cpu = parts.get(4).map(|value| value.to_string());
            let memory = parts.get(5).map(|value| value.to_string());
            Some(DataflowRuntimeInfo {
                id,
                name,
                status,
                nodes,
                cpu,
                memory,
            })
        })
        .collect()
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
