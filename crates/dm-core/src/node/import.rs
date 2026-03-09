use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use fs_extra::dir::{copy as dir_copy, CopyOptions};

use crate::events::{EventSource, OperationEvent};

use super::init::{init_dm_json, InitHints};
use super::model::Node;
use super::paths::node_dir;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitHubSource {
    repo_url: String,
    git_ref: Option<String>,
    repo_path: Option<String>,
}

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
    let source = parse_github_source(github_url)?;

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
            &source.repo_url,
            &temp_dir.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        bail!("Failed to clone repository");
    }

    if let Some(git_ref) = source.git_ref.as_deref() {
        let status = Command::new("git")
            .current_dir(&temp_dir)
            .args(["checkout", git_ref])
            .status()?;

        if !status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!("Failed to checkout ref '{}'", git_ref);
        }
    }

    if let Some(repo_path) = source.repo_path.as_deref() {
        let status = Command::new("git")
            .current_dir(&temp_dir)
            .args(["sparse-checkout", "set", repo_path])
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

        let src_path = temp_dir.join(repo_path);
        if src_path.is_dir() {
            let mut options = CopyOptions::new();
            options.content_only = true;
            if let Err(err) = dir_copy(&src_path, dest_dir, &options) {
                let _ = std::fs::remove_dir_all(&temp_dir);
                bail!("Failed to copy node files: {}", err);
            }
        } else {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!("Directory '{}' not found in the repository.", repo_path);
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

fn parse_github_source(github_url: &str) -> Result<GitHubSource> {
    if !github_url.starts_with("https://github.com/") {
        bail!("Invalid GitHub URL format: {}", github_url);
    }

    let parts: Vec<&str> = github_url.trim_end_matches('/').split('/').collect();
    if parts.len() < 5 {
        bail!("Invalid GitHub URL structure");
    }

    let repo_url = format!("https://github.com/{}/{}.git", parts[3], parts[4]);
    let (git_ref, repo_path) = if parts.len() > 6 && (parts[5] == "tree" || parts[5] == "blob") {
        let git_ref = parts[6].to_string();
        let repo_path = if parts.len() > 7 {
            Some(parts[7..].join("/"))
        } else {
            None
        };
        (Some(git_ref), repo_path)
    } else {
        (None, None)
    };

    Ok(GitHubSource {
        repo_url,
        git_ref,
        repo_path,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{import_git, import_local, node_dir, parse_github_source, GitHubSource};

    #[test]
    fn parse_github_source_supports_repo_root_url() {
        let source = parse_github_source("https://github.com/acme/project").unwrap();
        assert_eq!(
            source,
            GitHubSource {
                repo_url: "https://github.com/acme/project.git".to_string(),
                git_ref: None,
                repo_path: None,
            }
        );
    }

    #[test]
    fn parse_github_source_preserves_ref_and_subdir_for_tree_url() {
        let source =
            parse_github_source("https://github.com/acme/project/tree/release-1/examples/demo")
                .unwrap();
        assert_eq!(
            source,
            GitHubSource {
                repo_url: "https://github.com/acme/project.git".to_string(),
                git_ref: Some("release-1".to_string()),
                repo_path: Some("examples/demo".to_string()),
            }
        );
    }

    #[test]
    fn parse_github_source_rejects_non_github_urls() {
        let err = parse_github_source("https://example.com/acme/project")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Invalid GitHub URL format"));
    }

    #[test]
    fn import_local_rejects_missing_source_directory() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("missing-node");

        let err = import_local(home, "demo-node", &source)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Source directory"));
        assert!(!node_dir(home, "demo-node").exists());
    }

    #[test]
    fn import_local_rejects_duplicate_destination() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("source-node");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("README.md"), "# demo\n").unwrap();

        let existing = node_dir(home, "demo-node");
        fs::create_dir_all(&existing).unwrap();

        let err = import_local(home, "demo-node", &source)
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn import_local_copies_files_and_initializes_dm_json() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("source-node");
        fs::create_dir_all(source.join("pkg")).unwrap();
        fs::write(source.join("README.md"), "# Imported\n").unwrap();
        fs::write(
            source.join("pyproject.toml"),
            "[project]\nname = \"imported-node\"\nversion = \"1.2.3\"\ndescription = \"Imported\"\n",
        )
        .unwrap();
        fs::write(source.join("pkg/__init__.py"), "").unwrap();

        let node = import_local(home, "demo-node", &source).unwrap();
        let target = node_dir(home, "demo-node");

        assert_eq!(node.id, "demo-node");
        assert_eq!(node.name, "imported-node");
        assert_eq!(node.version, "1.2.3");
        assert!(target.join("README.md").exists());
        assert!(target.join("pkg/__init__.py").exists());
        assert!(target.join("dm.json").exists());
    }

    #[tokio::test]
    async fn import_git_rejects_duplicate_destination_before_fetch() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        fs::create_dir_all(node_dir(home, "demo-node")).unwrap();

        let err = import_git(home, "demo-node", "https://github.com/acme/project")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"));
    }

    #[tokio::test]
    async fn import_git_removes_destination_when_fetch_fails() {
        let dir = tempdir().unwrap();
        let home = dir.path();

        let err = import_git(home, "demo-node", "https://example.com/acme/project")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Failed to fetch source from GitHub"));
        assert!(!node_dir(home, "demo-node").exists());
    }
}
