use std::path::{Path, PathBuf};

use anyhow::Result;

pub(super) fn extract_tar(data: &[u8], target_dir: &Path) -> Result<()> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("tar")
        .args(["xzf", "-", "--strip-components=1", "-C"])
        .arg(target_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(data)?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let mut child = Command::new("tar")
            .args(["xzf", "-", "-C"])
            .arg(target_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(data)?;
        }
        let output2 = child.wait_with_output()?;
        if !output2.status.success() {
            let err = String::from_utf8_lossy(&output2.stderr);
            anyhow::bail!("tar extraction failed: {}", err);
        }
    }
    Ok(())
}

pub(super) fn extract_zip(data: &[u8], target_dir: &Path) -> Result<()> {
    use std::io::Cursor;

    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;
    archive.extract(target_dir)?;
    Ok(())
}

pub(super) fn find_dora_binary(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().map(|n| n == "dora").unwrap_or(false) {
                return Some(path);
            }
            if path.is_dir() {
                if path.file_name().map(|n| n == ".venv").unwrap_or(false) {
                    continue;
                }
                if let Some(found) = find_dora_binary(&path) {
                    return Some(found);
                }
            }
        }
    }
    None
}
