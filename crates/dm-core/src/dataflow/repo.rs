use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::model::{DataflowHistoryEntry, DataflowMeta, FlowMeta};
use super::paths::{
    dataflow_dir, dataflow_yaml_path, dataflows_dir, flow_config_path, flow_history_dir,
    flow_meta_path, flow_view_path, DATAFLOW_FILE,
};

pub fn list_projects(home: &Path) -> Result<Vec<DataflowMeta>> {
    let dir = dataflows_dir(home);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut dataflows = Vec::new();
    for entry in fs::read_dir(&dir).context("Failed to read dataflows directory")? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let yaml_path = dataflow_yaml_path(&path);
        if !yaml_path.exists() {
            continue;
        }

        let meta = entry.metadata()?;
        let size = fs::metadata(&yaml_path)
            .map(|value| value.len())
            .unwrap_or(0);
        let modified_at = match meta.modified() {
            Ok(t) => {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            }
            Err(_) => String::new(),
        };
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        dataflows.push(DataflowMeta {
            name: name.clone(),
            filename: format!("{name}/{DATAFLOW_FILE}"),
            modified_at,
            size,
        });
    }
    dataflows.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(dataflows)
}

pub fn read_yaml(home: &Path, name: &str) -> Result<String> {
    let path = dataflow_yaml_path(&dataflow_dir(home, name));
    fs::read_to_string(&path).with_context(|| format!("Failed to read dataflow '{}'", name))
}

pub fn write_yaml(home: &Path, name: &str, yaml: &str) -> Result<()> {
    let dir = dataflow_dir(home, name);
    fs::create_dir_all(&dir)?;
    initialize_flow_project(name, &dir)?;

    let path = dataflow_yaml_path(&dir);
    if let Ok(existing) = fs::read_to_string(&path) {
        if existing != yaml {
            write_history_snapshot(&dir, &existing)?;
        }
    }

    fs::write(&path, yaml).with_context(|| format!("Failed to save dataflow '{}'", name))?;
    touch_flow_meta(&dir, name)?;
    Ok(())
}

pub fn delete_project(home: &Path, name: &str) -> Result<()> {
    let path = dataflow_dir(home, name);
    fs::remove_dir_all(&path).with_context(|| format!("Failed to delete dataflow '{}'", name))
}

pub fn read_config(home: &Path, name: &str) -> Result<serde_json::Value> {
    let path = flow_config_path(&dataflow_dir(home, name));
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read dataflow config '{}'", name))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse dataflow config '{}'", name))
}

pub fn read_view(home: &Path, name: &str) -> Result<serde_json::Value> {
    let path = flow_view_path(&dataflow_dir(home, name));
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read view for '{}'", name))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse view for '{}'", name))
}

pub fn write_view(home: &Path, name: &str, view: &serde_json::Value) -> Result<()> {
    let dir = dataflow_dir(home, name);
    fs::create_dir_all(&dir)?;
    let path = flow_view_path(&dir);
    fs::write(
        &path,
        serde_json::to_string_pretty(view).context("Failed to serialize view.json")?,
    )
    .with_context(|| format!("Failed to write {}", path.display()))
}

pub fn write_config(home: &Path, name: &str, config: &serde_json::Value) -> Result<()> {
    let dir = dataflow_dir(home, name);
    fs::create_dir_all(&dir)?;
    initialize_flow_project(name, &dir)?;
    let config_path = flow_config_path(&dir);
    fs::write(
        &config_path,
        serde_json::to_string_pretty(config).context("Failed to serialize dataflow config")?,
    )
    .with_context(|| format!("Failed to write {}", config_path.display()))?;
    touch_flow_meta(&dir, name)
}

pub fn read_meta(home: &Path, name: &str) -> Result<FlowMeta> {
    let dir = dataflow_dir(home, name);
    let meta_path = flow_meta_path(&dir);
    let content = fs::read_to_string(&meta_path)
        .with_context(|| format!("Failed to read flow metadata '{}'", name))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse flow metadata '{}'", name))
}

pub fn write_meta(home: &Path, name: &str, meta: &FlowMeta) -> Result<()> {
    let dir = dataflow_dir(home, name);
    fs::create_dir_all(&dir)?;
    initialize_flow_project(name, &dir)?;

    let existing = read_meta(home, name).unwrap_or_default();
    let now = now_rfc3339();
    let merged = FlowMeta {
        id: if meta.id.is_empty() {
            name.to_string()
        } else {
            meta.id.clone()
        },
        name: if meta.name.is_empty() {
            name.to_string()
        } else {
            meta.name.clone()
        },
        description: meta.description.clone(),
        r#type: meta.r#type.clone(),
        tags: meta.tags.clone(),
        author: meta.author.clone(),
        cover: meta.cover.clone(),
        created_at: if existing.created_at.is_empty() {
            now.clone()
        } else {
            existing.created_at
        },
        updated_at: now,
    };

    let meta_path = flow_meta_path(&dir);
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&merged).context("Failed to serialize flow metadata")?,
    )
    .with_context(|| format!("Failed to write {}", meta_path.display()))
}

pub fn list_history_versions(home: &Path, name: &str) -> Result<Vec<DataflowHistoryEntry>> {
    let history_dir = flow_history_dir(&dataflow_dir(home, name));
    if !history_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&history_dir)
        .with_context(|| format!("Failed to read history for '{}'", name))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ext != "yml" && ext != "yaml" {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified_at = match metadata.modified() {
            Ok(t) => {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            }
            Err(_) => String::new(),
        };
        let version = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        entries.push(DataflowHistoryEntry {
            version,
            modified_at,
            size: metadata.len(),
        });
    }
    entries.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(entries)
}

pub fn read_history_version(home: &Path, name: &str, version: &str) -> Result<String> {
    let path = flow_history_dir(&dataflow_dir(home, name)).join(format!("{version}.yml"));
    fs::read_to_string(&path).with_context(|| {
        format!(
            "Failed to read history version '{}' for dataflow '{}'",
            version, name
        )
    })
}

pub fn restore_history_version(home: &Path, name: &str, version: &str) -> Result<()> {
    let content = read_history_version(home, name, version)?;
    write_yaml(home, name, &content)
}

pub fn migrate_legacy_layout(home: &Path) -> Result<usize> {
    let dir = dataflows_dir(home);
    if !dir.exists() {
        return Ok(0);
    }

    let mut migrated = 0;
    for entry in fs::read_dir(&dir).context("Failed to read dataflows directory")? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ext != "yml" && ext != "yaml" {
            continue;
        }

        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read legacy dataflow '{}'", name))?;
        let project_dir = dataflow_dir(home, &name);
        fs::create_dir_all(&project_dir)?;
        initialize_flow_project(&name, &project_dir)?;
        fs::write(dataflow_yaml_path(&project_dir), content)
            .with_context(|| format!("Failed to migrate dataflow '{}'", name))?;
        touch_flow_meta(&project_dir, &name)?;
        fs::remove_file(&path)
            .with_context(|| format!("Failed to remove legacy dataflow '{}'", name))?;
        migrated += 1;
    }

    Ok(migrated)
}

pub fn initialize_flow_project(name: &str, dir: &Path) -> Result<()> {
    fs::create_dir_all(flow_history_dir(dir))?;

    let meta_path = flow_meta_path(dir);
    if !meta_path.exists() {
        let now = now_rfc3339();
        let meta = FlowMeta {
            id: name.to_string(),
            name: name.to_string(),
            created_at: now.clone(),
            updated_at: now,
            ..Default::default()
        };
        fs::write(
            &meta_path,
            serde_json::to_string_pretty(&meta).context("Failed to serialize flow.json")?,
        )
        .with_context(|| format!("Failed to write {}", meta_path.display()))?;
    }

    let config_path = flow_config_path(dir);
    if !config_path.exists() {
        fs::write(&config_path, "{}\n")
            .with_context(|| format!("Failed to write {}", config_path.display()))?;
    }

    Ok(())
}

pub fn touch_flow_meta(dir: &Path, name: &str) -> Result<()> {
    let meta_path = flow_meta_path(dir);
    let now = now_rfc3339();
    let mut meta = if meta_path.exists() {
        let content = fs::read_to_string(&meta_path)
            .with_context(|| format!("Failed to read {}", meta_path.display()))?;
        serde_json::from_str::<FlowMeta>(&content)
            .with_context(|| format!("Failed to parse {}", meta_path.display()))?
    } else {
        FlowMeta {
            id: name.to_string(),
            name: name.to_string(),
            created_at: now.clone(),
            ..Default::default()
        }
    };

    if meta.id.is_empty() {
        meta.id = name.to_string();
    }
    if meta.name.is_empty() {
        meta.name = name.to_string();
    }
    if meta.created_at.is_empty() {
        meta.created_at = now.clone();
    }
    meta.updated_at = now;

    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).context("Failed to serialize flow.json")?,
    )
    .with_context(|| format!("Failed to write {}", meta_path.display()))
}

pub fn load_flow_config_for_yaml(home: &Path, yaml_path: &Path) -> Option<serde_json::Value> {
    let parent = yaml_path.parent()?;
    let dataflows_root = dataflows_dir(home);
    if !parent.starts_with(&dataflows_root) {
        return None;
    }
    let path = flow_config_path(parent);
    if !path.exists() {
        return Some(serde_json::json!({}));
    }

    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn select_flow_node_config(
    flow_config: &serde_json::Value,
    yaml_node_id: &str,
    node_id: &str,
) -> serde_json::Value {
    flow_config
        .get(yaml_node_id)
        .cloned()
        .or_else(|| flow_config.get(node_id).cloned())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn write_history_snapshot(dir: &Path, content: &str) -> Result<()> {
    let history_dir = flow_history_dir(dir);
    fs::create_dir_all(&history_dir)?;
    let version_id = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let history_path = history_dir.join(format!("{version_id}.yml"));
    fs::write(&history_path, content).with_context(|| {
        format!(
            "Failed to write history snapshot '{}'",
            history_path.display()
        )
    })
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
