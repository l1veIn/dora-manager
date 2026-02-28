use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use fs_extra::dir::{copy as dir_copy, CopyOptions};

use crate::events::{EventSource, OperationEvent};
use crate::registry::{self, NodeMeta};

use super::model::NodeMetaFile;
use super::paths::{dm_json_path, node_dir};
use super::{current_timestamp, NodeEntry, NodeSource};

pub async fn download_node(home: &Path, id: &str) -> Result<NodeEntry> {
    let op = OperationEvent::new(home, EventSource::Core, "node.download").attr("node_id", id);
    op.emit_start();

    let result = async {
        let node_path = node_dir(home, id);
        if node_path.exists() {
            bail!("Node '{}' already exists at {}", id, node_path.display());
        }

        let registry = registry::fetch_registry()
            .await
            .context("Failed to fetch registry")?;

        let meta = registry::find_node(&registry, id)
            .ok_or_else(|| anyhow::anyhow!("Node '{}' not found in registry", id))?;

        download_registry_node(home, id, meta).await
    }
    .await;

    op.emit_result(&result);
    result
}

pub(crate) async fn download_registry_node(
    home: &Path,
    id: &str,
    meta: &NodeMeta,
) -> Result<NodeEntry> {
    let node_path = node_dir(home, id);
    if node_path.exists() {
        bail!("Node '{}' already exists at {}", id, node_path.display());
    }

    std::fs::create_dir_all(&node_path)
        .with_context(|| format!("Failed to create directory: {}", node_path.display()))?;

    let dm_meta = NodeMetaFile {
        id: id.to_string(),
        name: meta.name.clone(),
        version: String::new(),
        installed_at: current_timestamp(),
        source: NodeSource {
            build: meta.build.clone(),
            github: meta.github.clone(),
        },
        description: meta.description.clone(),
        executable: String::new(),
        author: None,
        category: meta.category.clone(),
        inputs: meta.inputs.clone(),
        outputs: meta.outputs.clone(),
        avatar: None,
        config_schema: None,
    };

    let dm_path = dm_json_path(home, id);
    let dm_json = serde_json::to_string_pretty(&dm_meta).context("Failed to serialize dm.json")?;
    std::fs::write(&dm_path, dm_json)
        .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

    let repo_url = meta.source.as_ref().or(meta.github.as_ref());
    if let Some(github_url) = repo_url {
        if let Err(err) = download_github_source(github_url, &node_path).await {
            let _ = std::fs::remove_dir_all(&node_path);
            bail!("Failed to fetch source from GitHub: {}", err);
        }
    }

    Ok(dm_meta.into_entry(node_path))
}

async fn download_github_source(github_url: &str, dest_dir: &Path) -> Result<()> {
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
                bail!("Failed to copy node files via fs_extra: {}", err);
            }
        } else {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!(
                "The specified node directory '{}' was not found in the repository.",
                folder_path
            );
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
            bail!("Failed to copy node files via fs_extra: {}", err);
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_dir_all(dest_dir.join(".git"));
    Ok(())
}
