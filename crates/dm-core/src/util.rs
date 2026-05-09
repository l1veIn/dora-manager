use std::path::Path;

/// Check if a command exists in PATH, returns its full path.
pub fn check_command(name: &str) -> Option<String> {
    which::which(name)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
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
