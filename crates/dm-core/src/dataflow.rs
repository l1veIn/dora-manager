use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::events::{EventSource, OperationEvent};
use crate::node::{NodeMetaFile, node_dir, meta_path};

// ─── Dataflow Management ───

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataflowMeta {
    pub name: String,
    pub filename: String,
    pub modified_at: String,
    pub size: u64,
}

/// List all saved dataflows in `~/.dm/dataflows/`
pub fn list(home: &Path) -> Result<Vec<DataflowMeta>> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.list");
    op.emit_start();
    
    let result = (|| -> Result<Vec<DataflowMeta>> {
        let dir = home.join("dataflows");
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut dataflows = Vec::new();
        for entry in fs::read_dir(&dir).context("Failed to read dataflows directory")? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yml" || ext == "yaml" {
                        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        let meta = entry.metadata()?;
                        let size = meta.len();
                        let modified_at = match meta.modified() {
                            Ok(t) => {
                                let dt: chrono::DateTime<chrono::Utc> = t.into();
                                dt.to_rfc3339()
                            }
                            Err(_) => "".to_string(),
                        };
                        dataflows.push(DataflowMeta {
                            name,
                            filename,
                            modified_at,
                            size,
                        });
                    }
                }
            }
        }
        dataflows.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        Ok(dataflows)
    })();
    
    op.emit_result(&result);
    result
}

/// Get a single dataflow's YAML content
pub fn get(home: &Path, name: &str) -> Result<String> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.get")
        .attr("name", name);
    op.emit_start();
    
    let result = {
        let path = home.join("dataflows").join(format!("{}.yml", name));
        fs::read_to_string(&path)
            .with_context(|| format!("Failed to read dataflow '{}'", name))
    };
    
    op.emit_result(&result);
    result
}

/// Save (create or update) a dataflow's YAML content
pub fn save(home: &Path, name: &str, yaml: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.save")
        .attr("name", name);
    op.emit_start();
    
    let result = (|| {
        let dir = home.join("dataflows");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.yml", name));
        fs::write(&path, yaml)
            .with_context(|| format!("Failed to save dataflow '{}'", name))
    })();
    
    op.emit_result(&result);
    result
}

/// Delete a dataflow file
pub fn delete(home: &Path, name: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.delete")
        .attr("name", name);
    op.emit_start();
    
    let result = {
        let path = home.join("dataflows").join(format!("{}.yml", name));
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete dataflow '{}'", name))
    };
    
    op.emit_result(&result);
    result
}

// ─── Dataflow Execution ───

/// Transpile a dataflow yaml file, replacing DM node IDs in `path:` fields
/// with absolute paths to the installed node executables.
///
/// Each managed node has a `meta.json` containing an `executable` field
/// (e.g., `.venv/bin/dora-vad`). Transpile resolves this to an absolute path.
pub fn transpile_graph(home: &Path, yaml_path: &Path) -> Result<serde_yaml::Value> {
    let op = OperationEvent::new(home, EventSource::Dataflow, "dataflow.transpile")
        .attr("path", yaml_path.display().to_string());
    op.emit_start();

    let result = (|| {
        let content = fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read graph yaml at {}", yaml_path.display()))?;

        let mut graph: serde_yaml::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse yaml at {}", yaml_path.display()))?;

        if let Some(nodes) = graph.get_mut("nodes").and_then(|n| n.as_sequence_mut()) {
            for node in nodes {
                if let Some(path_val) = node.get("path").and_then(|p| p.as_str()) {
                    let node_cache_dir = node_dir(home, path_val);
                    let meta_file_path = meta_path(home, path_val);

                    if node_cache_dir.exists() && meta_file_path.exists() {
                        let meta_content = fs::read_to_string(&meta_file_path).unwrap_or_default();
                        if let Ok(meta) = serde_json::from_str::<NodeMetaFile>(&meta_content) {
                            if !meta.executable.is_empty() {
                                let abs_exec = node_cache_dir.join(&meta.executable);
                                let node_map = node.as_mapping_mut().unwrap();
                                node_map.remove(serde_yaml::Value::String("path".to_string()));
                                node_map.insert(
                                    serde_yaml::Value::String("path".to_string()),
                                    serde_yaml::Value::String(abs_exec.display().to_string()),
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(graph)
    })();

    op.emit_result(&result);
    result
}

