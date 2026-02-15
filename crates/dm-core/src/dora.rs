use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::config;

/// Resolve the path to the active dora binary managed by dm.
pub fn active_dora_bin(home: &PathBuf) -> Result<PathBuf> {
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
    home: &PathBuf,
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

/// Run dora with inherited stdio (for interactive / streaming commands).
pub async fn exec_dora(home: &PathBuf, args: &[String], verbose: bool) -> Result<i32> {
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

/// Get the version string from a dora binary.
pub async fn get_dora_version(bin_path: &PathBuf) -> Result<String> {
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
