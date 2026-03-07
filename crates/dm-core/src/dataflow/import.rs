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
        0 => anyhow::bail!("No dataflow.yml or *.yml file found in '{}'", source.display()),
        _ => anyhow::bail!(
            "Multiple YAML files found in '{}'; expected a dataflow project directory",
            source.display()
        ),
    }
}

async fn clone_github_source(github_url: &str, dest_dir: &Path) -> Result<()> {
    if !github_url.starts_with("https://github.com/") {
        anyhow::bail!("Invalid GitHub URL format: {}", github_url);
    }

    let parts: Vec<&str> = github_url.split('/').collect();
    if parts.len() < 5 {
        anyhow::bail!("Invalid GitHub URL structure");
    }

    let repo_url = format!("https://github.com/{}/{}.git", parts[3], parts[4]);
    let mut repo_path = String::new();
    if parts.len() > 7 && (parts[5] == "tree" || parts[5] == "blob") {
        repo_path = parts[7..].join("/");
    }

    let clone_root = dest_dir.join("repo");
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            &repo_url,
            &clone_root.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to clone repository");
    }

    if !repo_path.is_empty() {
        let is_file_path = repo_path.ends_with(".yml") || repo_path.ends_with(".yaml");
        let normalized_path = if is_file_path {
            format!("/{}", repo_path.trim_start_matches('/'))
        } else {
            repo_path.clone()
        };
        let args = if is_file_path {
            vec!["sparse-checkout", "set", "--no-cone", normalized_path.as_str()]
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

    let source_path = if repo_path.is_empty() {
        clone_root
    } else {
        clone_root.join(repo_path)
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
