use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use fs_extra::dir::{copy as dir_copy, CopyOptions};

use crate::events::{EventSource, OperationEvent};

use super::init::{init_dm_json, InitHints};
use super::model::Node;
use super::paths::node_dir;

/// Import a node from a local directory (copy to ~/.dm/nodes/).
pub fn import_local(home: &Path, id: &str, source_dir: &Path) -> Result<Node> {
    let op = OperationEvent::new(home, EventSource::Core, "node.import_local")
        .attr("node_id", id)
        .attr("source", source_dir.display().to_string());
    op.emit_start();

    let result = (|| {
        let node_path = node_dir(home, id);
        if node_path.exists() {
            bail!("Node '{}' already exists at {}", id, node_path.display());
        }

        if !source_dir.exists() || !source_dir.is_dir() {
            bail!("Source directory '{}' not found", source_dir.display());
        }

        std::fs::create_dir_all(&node_path)
            .with_context(|| format!("Failed to create directory: {}", node_path.display()))?;

        let mut options = CopyOptions::new();
        options.content_only = true;
        dir_copy(source_dir, &node_path, &options).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                source_dir.display(),
                node_path.display()
            )
        })?;

        init_dm_json(id, &node_path, InitHints::default())
    })();

    op.emit_result(&result);
    result
}

/// Import a node from a git URL (clone to ~/.dm/nodes/).
pub async fn import_git(home: &Path, id: &str, git_url: &str) -> Result<Node> {
    let op = OperationEvent::new(home, EventSource::Core, "node.import_git")
        .attr("node_id", id)
        .attr("url", git_url);
    op.emit_start();

    let result = async {
        let node_path = node_dir(home, id);
        if node_path.exists() {
            bail!("Node '{}' already exists at {}", id, node_path.display());
        }

        std::fs::create_dir_all(&node_path)
            .with_context(|| format!("Failed to create directory: {}", node_path.display()))?;

        if let Err(err) = clone_github_source(git_url, &node_path).await {
            let _ = std::fs::remove_dir_all(&node_path);
            bail!("Failed to fetch source from GitHub: {}", err);
        }

        init_dm_json(id, &node_path, InitHints::default())
    }
    .await;

    op.emit_result(&result);
    result
}

// ─── Git clone helper ───

async fn clone_github_source(github_url: &str, dest_dir: &Path) -> Result<()> {
    if !github_url.starts_with("https://github.com/") {
        bail!("Invalid GitHub URL format: {}", github_url);
    }

    let parts: Vec<&str> = github_url.split('/').collect();
    if parts.len() < 5 {
        bail!("Invalid GitHub URL structure");
    }

    let repo_url = format!("https://github.com/{}/{}.git", parts[3], parts[4]);
    let mut folder_path = String::new();
    if parts.len() > 7 && (parts[5] == "tree" || parts[5] == "blob") {
        folder_path = parts[7..].join("/");
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("dm_clone_{nanos}"));

    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            &repo_url,
            &temp_dir.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        bail!("Failed to clone repository");
    }

    if !folder_path.is_empty() {
        let status = Command::new("git")
            .current_dir(&temp_dir)
            .args(["sparse-checkout", "set", &folder_path])
            .status()?;

        if !status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!("Failed to set sparse-checkout");
        }

        let status = Command::new("git")
            .current_dir(&temp_dir)
            .arg("checkout")
            .status()?;

        if !status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!("Failed to checkout sparse files");
        }

        let src_path = temp_dir.join(&folder_path);
        if src_path.exists() {
            let mut options = CopyOptions::new();
            options.content_only = true;
            if let Err(err) = dir_copy(&src_path, dest_dir, &options) {
                let _ = std::fs::remove_dir_all(&temp_dir);
                bail!("Failed to copy node files: {}", err);
            }
        } else {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!("Directory '{}' not found in the repository.", folder_path);
        }
    } else {
        let mut options = CopyOptions::new();
        options.content_only = true;

        let git_dir = temp_dir.join(".git");
        if git_dir.exists() {
            let _ = std::fs::remove_dir_all(&git_dir);
        }

        if let Err(err) = dir_copy(&temp_dir, dest_dir, &options) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!("Failed to copy node files: {}", err);
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_dir_all(dest_dir.join(".git"));
    Ok(())
}
