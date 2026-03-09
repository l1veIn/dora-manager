use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use fs_extra::dir::{copy as dir_copy, CopyOptions};

use super::paths::{
    dataflow_dir, dataflow_yaml_path, flow_config_path, flow_meta_path, FLOW_CONFIG_FILE,
    FLOW_META_FILE,
};
use super::repo::{initialize_flow_project, touch_flow_meta, write_yaml};

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitHubSource {
    repo_url: String,
    git_ref: Option<String>,
    repo_path: Option<String>,
}

pub fn infer_import_name(source: &str) -> String {
    if source.starts_with("https://") || source.starts_with("http://") {
        let trimmed = source.trim_end_matches('/');
        let last = trimmed.rsplit('/').next().unwrap_or("dataflow");
        let name = last
            .strip_suffix(".yml")
            .or_else(|| last.strip_suffix(".yaml"))
            .unwrap_or(last);
        return name.to_string();
    }

    let path = Path::new(source);
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    file_name
        .strip_suffix(".yml")
        .or_else(|| file_name.strip_suffix(".yaml"))
        .unwrap_or(&file_name)
        .to_string()
}

pub fn import_local(home: &Path, name: &str, source: &Path) -> Result<()> {
    if !source.exists() {
        anyhow::bail!("Source '{}' not found", source.display());
    }
    let project_dir = dataflow_dir(home, name);
    if project_dir.exists() {
        anyhow::bail!("Dataflow '{}' already exists", name);
    }

    if source.is_file() {
        import_local_file(home, name, source)
    } else if source.is_dir() {
        import_local_dir(home, name, source)
    } else {
        anyhow::bail!("Unsupported source '{}'", source.display());
    }
}

pub async fn import_git(home: &Path, name: &str, git_url: &str) -> Result<()> {
    let project_dir = dataflow_dir(home, name);
    if project_dir.exists() {
        anyhow::bail!("Dataflow '{}' already exists", name);
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("dm_dataflow_clone_{nanos}"));
    fs::create_dir_all(&temp_dir)?;

    if let Err(err) = clone_github_source(git_url, &temp_dir).await {
        let _ = fs::remove_dir_all(&temp_dir);
        anyhow::bail!("Failed to fetch source from GitHub: {}", err);
    }

    let source = resolve_import_source(&temp_dir)?;
    let result = if source.is_file() {
        import_local_file(home, name, &source)
    } else {
        import_local_dir(home, name, &source)
    };

    let _ = fs::remove_dir_all(&temp_dir);
    result
}

fn import_local_file(home: &Path, name: &str, source: &Path) -> Result<()> {
    let content = fs::read_to_string(source)
        .with_context(|| format!("Failed to read dataflow file '{}'", source.display()))?;
    write_yaml(home, name, &content)
}

fn import_local_dir(home: &Path, name: &str, source: &Path) -> Result<()> {
    let source_yaml = resolve_import_source(source)?;
    let project_dir = dataflow_dir(home, name);
    fs::create_dir_all(&project_dir)?;
    initialize_flow_project(name, &project_dir)?;

    let yaml_target = dataflow_yaml_path(&project_dir);
    fs::copy(&source_yaml, &yaml_target).with_context(|| {
        format!(
            "Failed to copy dataflow file '{}' to '{}'",
            source_yaml.display(),
            yaml_target.display()
        )
    })?;

    let source_config = source.join(FLOW_CONFIG_FILE);
    if source_config.exists() {
        fs::copy(&source_config, flow_config_path(&project_dir))
            .with_context(|| format!("Failed to copy '{}'", source_config.display()))?;
    }

    let source_meta = source.join(FLOW_META_FILE);
    if source_meta.exists() {
        fs::copy(&source_meta, flow_meta_path(&project_dir))
            .with_context(|| format!("Failed to copy '{}'", source_meta.display()))?;
    }
    touch_flow_meta(&project_dir, name)?;
    Ok(())
}

fn resolve_import_source(source: &Path) -> Result<PathBuf> {
    if source.is_file() {
        return Ok(source.to_path_buf());
    }

    let dataflow = source.join(super::paths::DATAFLOW_FILE);
    if dataflow.exists() {
        return Ok(dataflow);
    }

    let mut yaml_files = Vec::new();
    for entry in fs::read_dir(source)
        .with_context(|| format!("Failed to read directory '{}'", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ext == "yml" || ext == "yaml" {
            yaml_files.push(path);
        }
    }

    match yaml_files.len() {
        1 => Ok(yaml_files.remove(0)),
        0 => anyhow::bail!(
            "No dataflow.yml or *.yml file found in '{}'",
            source.display()
        ),
        _ => anyhow::bail!(
            "Multiple YAML files found in '{}'; expected a dataflow project directory",
            source.display()
        ),
    }
}

async fn clone_github_source(github_url: &str, dest_dir: &Path) -> Result<()> {
    let source = parse_github_source(github_url)?;
    let clone_args = build_clone_args(&source, &dest_dir.join("repo"));

    let clone_root = dest_dir.join("repo");
    let status = Command::new("git").args(&clone_args).status()?;

    if !status.success() {
        anyhow::bail!("Failed to clone repository");
    }

    if let Some(repo_path) = source.repo_path.as_deref() {
        let is_file_path = repo_path.ends_with(".yml") || repo_path.ends_with(".yaml");
        let normalized_path = if is_file_path {
            format!("/{}", repo_path.trim_start_matches('/'))
        } else {
            repo_path.to_string()
        };
        let args = if is_file_path {
            vec![
                "sparse-checkout",
                "set",
                "--no-cone",
                normalized_path.as_str(),
            ]
        } else {
            vec!["sparse-checkout", "set", normalized_path.as_str()]
        };
        let status = Command::new("git")
            .current_dir(&clone_root)
            .args(&args)
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to set sparse-checkout");
        }

        let status = Command::new("git")
            .current_dir(&clone_root)
            .arg("checkout")
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to checkout sparse files");
        }
    }

    let source_path = if let Some(repo_path) = source.repo_path.as_deref() {
        clone_root.join(repo_path)
    } else {
        clone_root
    };
    if !source_path.exists() {
        anyhow::bail!("Path '{}' not found in repository", github_url);
    }

    let mut options = CopyOptions::new();
    options.content_only = true;
    if source_path.is_dir() {
        dir_copy(&source_path, dest_dir, &options).with_context(|| {
            format!(
                "Failed to copy repository contents from '{}'",
                source_path.display()
            )
        })?;
    } else {
        let file_name = source_path.file_name().unwrap_or_default();
        fs::copy(&source_path, dest_dir.join(file_name))
            .with_context(|| format!("Failed to copy file '{}'", source_path.display()))?;
    }

    let _ = fs::remove_dir_all(dest_dir.join(".git"));
    let _ = fs::remove_dir_all(dest_dir.join("repo"));
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
        anyhow::bail!("Invalid GitHub URL format: {}", github_url);
    }

    let parts: Vec<&str> = github_url.trim_end_matches('/').split('/').collect();
    if parts.len() < 5 {
        anyhow::bail!("Invalid GitHub URL structure");
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
    use std::path::Path;

    use serde_json::Value;
    use tempfile::tempdir;

    use crate::dataflow::paths::DATAFLOW_FILE;

    use super::{
        build_clone_args, flow_config_path, flow_meta_path, import_git, import_local,
        infer_import_name, parse_github_source, GitHubSource, FLOW_CONFIG_FILE, FLOW_META_FILE,
    };

    #[test]
    fn infer_import_name_from_github_blob_url() {
        let name = infer_import_name(
            "https://github.com/l1veIn/dora-manager/blob/master/tests/dataflows/system-test-full.yml",
        );
        assert_eq!(name, "system-test-full");
    }

    #[test]
    fn parse_github_source_preserves_ref_and_path_for_blob_url() {
        let source = parse_github_source(
            "https://github.com/acme/project/blob/feature-x/tests/dataflows/demo.yml",
        )
        .unwrap();
        assert_eq!(
            source,
            GitHubSource {
                repo_url: "https://github.com/acme/project.git".to_string(),
                git_ref: Some("feature-x".to_string()),
                repo_path: Some("tests/dataflows/demo.yml".to_string()),
            }
        );
    }

    #[test]
    fn parse_github_source_preserves_ref_and_path_for_tree_url() {
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
    fn parse_github_source_rejects_non_github_urls() {
        let err = parse_github_source("https://example.com/acme/project")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Invalid GitHub URL format"));
    }

    #[test]
    fn build_clone_args_requests_explicit_ref_when_present() {
        let source = GitHubSource {
            repo_url: "https://github.com/acme/project.git".to_string(),
            git_ref: Some("feature-x".to_string()),
            repo_path: Some("examples/demo".to_string()),
        };

        let args = build_clone_args(&source, Path::new("/tmp/repo"));
        assert_eq!(
            args,
            vec![
                "clone",
                "--depth",
                "1",
                "--filter=blob:none",
                "--sparse",
                "--branch",
                "feature-x",
                "--single-branch",
                "https://github.com/acme/project.git",
                "/tmp/repo",
            ]
        );
    }

    #[test]
    fn import_local_rejects_missing_source() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("missing.yml");

        let err = import_local(home, "demo-flow", &source)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Source"));
    }

    #[test]
    fn import_local_rejects_duplicate_destination() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("source.yml");
        fs::write(&source, "nodes: []\n").unwrap();
        fs::create_dir_all(home.join("dataflows/demo-flow")).unwrap();

        let err = import_local(home, "demo-flow", &source)
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn import_local_dir_copies_config_and_meta() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("source-flow");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join(DATAFLOW_FILE), "nodes:\n  - id: a\n").unwrap();
        fs::write(source.join(FLOW_CONFIG_FILE), "{\n  \"a\": 1\n}\n").unwrap();
        fs::write(
            source.join(FLOW_META_FILE),
            concat!(
                "{\n",
                "  \"id\": \"source-flow\",\n",
                "  \"name\": \"source-flow\",\n",
                "  \"description\": \"Imported flow\",\n",
                "  \"type\": \"demo\",\n",
                "  \"tags\": [],\n",
                "  \"created_at\": \"2026-03-09T00:00:00Z\",\n",
                "  \"updated_at\": \"2026-03-09T00:00:00Z\"\n",
                "}\n"
            ),
        )
        .unwrap();

        import_local(home, "demo-flow", &source).unwrap();

        let project_dir = home.join("dataflows/demo-flow");
        assert_eq!(
            fs::read_to_string(project_dir.join(DATAFLOW_FILE)).unwrap(),
            "nodes:\n  - id: a\n"
        );
        assert_eq!(
            fs::read_to_string(flow_config_path(&project_dir)).unwrap(),
            "{\n  \"a\": 1\n}\n"
        );

        let meta: Value =
            serde_json::from_str(&fs::read_to_string(flow_meta_path(&project_dir)).unwrap())
                .unwrap();
        assert_eq!(meta["description"], "Imported flow");
        assert_eq!(meta["id"], "source-flow");
        assert_eq!(meta["name"], "source-flow");
    }

    #[test]
    fn import_local_dir_accepts_single_yaml_without_dataflow_file_name() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("source-flow");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("custom.yaml"), "nodes:\n  - id: a\n").unwrap();

        import_local(home, "demo-flow", &source).unwrap();

        let project_dir = home.join("dataflows/demo-flow");
        assert_eq!(
            fs::read_to_string(project_dir.join(DATAFLOW_FILE)).unwrap(),
            "nodes:\n  - id: a\n"
        );
    }

    #[test]
    fn import_local_dir_rejects_multiple_yaml_files() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("source-flow");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.yml"), "nodes: []\n").unwrap();
        fs::write(source.join("b.yaml"), "nodes: []\n").unwrap();

        let err = import_local(home, "demo-flow", &source)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Multiple YAML files found"));
        assert!(!home.join("dataflows/demo-flow").exists());
    }

    #[test]
    fn import_local_dir_rejects_directory_without_yaml() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let source = home.join("source-flow");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("README.md"), "# demo\n").unwrap();

        let err = import_local(home, "demo-flow", &source)
            .unwrap_err()
            .to_string();
        assert!(err.contains("No dataflow.yml or *.yml file found"));
    }

    #[tokio::test]
    async fn import_git_rejects_duplicate_destination_before_fetch() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        fs::create_dir_all(home.join("dataflows/demo-flow")).unwrap();

        let err = import_git(home, "demo-flow", "https://github.com/acme/project")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"));
    }

    #[tokio::test]
    async fn import_git_cleans_up_destination_when_fetch_fails() {
        let dir = tempdir().unwrap();
        let home = dir.path();

        let err = import_git(home, "demo-flow", "https://example.com/acme/project")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Failed to fetch source from GitHub"));
        assert!(!home.join("dataflows/demo-flow").exists());
    }
}
