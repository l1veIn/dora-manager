use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use fs_extra::dir::{copy as dir_copy, CopyOptions};

use crate::events::{EventSource, OperationEvent};

use super::local::load_service_from_dir;
use super::model::Service;
use super::paths::service_dir;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitHubSource {
    repo_url: String,
    git_ref: Option<String>,
    repo_path: Option<String>,
}

pub fn import_local(home: &Path, id: &str, source_dir: &Path) -> Result<Service> {
    let op = OperationEvent::new(home, EventSource::Core, "service.import_local")
        .attr("service_id", id)
        .attr("source", source_dir.display().to_string());
    op.emit_start();

    let result = (|| {
        let service_path = service_dir(home, id);
        if service_path.exists() {
            bail!(
                "Service '{}' already exists at {}",
                id,
                service_path.display()
            );
        }

        if !source_dir.exists() || !source_dir.is_dir() {
            bail!("Source directory '{}' not found", source_dir.display());
        }

        std::fs::create_dir_all(&service_path)
            .with_context(|| format!("Failed to create directory: {}", service_path.display()))?;

        let mut options = CopyOptions::new();
        options.content_only = true;
        if let Err(err) = dir_copy(source_dir, &service_path, &options) {
            let _ = std::fs::remove_dir_all(&service_path);
            bail!(
                "Failed to copy {} to {}: {}",
                source_dir.display(),
                service_path.display(),
                err
            );
        }

        validate_imported_service(id, &service_path)
    })();

    op.emit_result(&result);
    result
}

pub async fn import_git(home: &Path, id: &str, git_url: &str) -> Result<Service> {
    let op = OperationEvent::new(home, EventSource::Core, "service.import_git")
        .attr("service_id", id)
        .attr("url", git_url);
    op.emit_start();

    let result = async {
        let service_path = service_dir(home, id);
        if service_path.exists() {
            bail!(
                "Service '{}' already exists at {}",
                id,
                service_path.display()
            );
        }

        std::fs::create_dir_all(&service_path)
            .with_context(|| format!("Failed to create directory: {}", service_path.display()))?;

        if let Err(err) = clone_github_source(git_url, &service_path).await {
            let _ = std::fs::remove_dir_all(&service_path);
            bail!("Failed to fetch service source from GitHub: {}", err);
        }

        validate_imported_service(id, &service_path)
    }
    .await;

    op.emit_result(&result);
    result
}

fn validate_imported_service(id: &str, service_path: &Path) -> Result<Service> {
    let service = load_service_from_dir(service_path)?;
    if service.id != id {
        let _ = std::fs::remove_dir_all(service_path);
        bail!(
            "Imported service id '{}' does not match requested id '{}'",
            service.id,
            id
        );
    }
    Ok(service.with_path(service_path.to_path_buf()))
}

async fn clone_github_source(github_url: &str, dest_dir: &Path) -> Result<()> {
    let source = parse_github_source(github_url)?;

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("dm_service_clone_{nanos}"));
    let clone_args = build_clone_args(&source, &temp_dir);

    let status = Command::new("git").args(&clone_args).status()?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        bail!("Failed to clone repository");
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
            copy_dir_content(&src_path, dest_dir)?;
        } else {
            let _ = std::fs::remove_dir_all(&temp_dir);
            bail!("Directory '{}' not found in the repository.", repo_path);
        }
    } else {
        let git_dir = temp_dir.join(".git");
        if git_dir.exists() {
            let _ = std::fs::remove_dir_all(&git_dir);
        }
        copy_dir_content(&temp_dir, dest_dir)?;
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_dir_all(dest_dir.join(".git"));
    Ok(())
}

fn copy_dir_content(source: &Path, dest: &Path) -> Result<()> {
    let mut options = CopyOptions::new();
    options.content_only = true;
    dir_copy(source, dest, &options)
        .with_context(|| format!("Failed to copy service files from {}", source.display()))?;
    Ok(())
}

fn build_clone_args(source: &GitHubSource, clone_root: &Path) -> Vec<String> {
    let mut args = vec![
        "clone".to_string(),
        "--depth".to_string(),
        "1".to_string(),
        "--filter=blob:none".to_string(),
        "--sparse".to_string(),
    ];

    if let Some(git_ref) = source.git_ref.as_deref() {
        args.push("--branch".to_string());
        args.push(git_ref.to_string());
        args.push("--single-branch".to_string());
    }

    args.push(source.repo_url.clone());
    args.push(clone_root.to_string_lossy().to_string());
    args
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
