use std::path::{Path, PathBuf};

pub const DM_CLI_BIN_ENV_KEY: &str = "DM_CLI_BIN";

/// Check if a command exists in PATH, returns its full path.
pub fn check_command(name: &str) -> Option<String> {
    which::which(name)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Resolve the `dm` CLI binary used to launch hidden bridge nodes.
///
/// Priority is:
/// 1. `DM_CLI_BIN`
/// 2. `dm` found in `PATH`
/// 3. a `dm` executable next to the current process
/// 4. literal `dm` as a last-resort runtime command
pub fn resolve_dm_cli_exe() -> PathBuf {
    if let Ok(override_path) = std::env::var(DM_CLI_BIN_ENV_KEY) {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    resolve_dm_cli_exe_from_path_or_sibling().unwrap_or_else(|| PathBuf::from("dm"))
}

/// Resolve an installed `dm` CLI binary without falling back to a bare command name.
pub fn resolve_dm_cli_exe_from_path_or_sibling() -> Option<PathBuf> {
    which::which("dm").ok().or_else(|| {
        std::env::current_exe().ok().and_then(|exe| {
            let dm_path = exe.parent()?.join("dm");
            dm_path.exists().then_some(dm_path)
        })
    })
}

/// Get a command's version output.
pub async fn get_command_version(cmd: &str, args: &[&str]) -> Option<String> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .ok()?;
    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if out.is_empty() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.is_empty() {
            None
        } else {
            Some(err)
        }
    } else {
        Some(out)
    }
}

/// Human-readable file size
pub fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Check if a path is a valid dora binary (exists and is executable)
pub fn is_valid_dora_binary(path: &Path) -> bool {
    path.exists() && path.is_file()
}
