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

/// Transpile a dataflow yaml file, replacing local references to installed packages
/// with their actual sandbox execution paths.
pub fn transpile_graph(home: &Path, yaml_path: &Path) -> Result<serde_yaml::Value> {
    let op = OperationEvent::new(home, EventSource::Dataflow, "dataflow.transpile")
        .attr("path", yaml_path.display().to_string());
    op.emit_start();

    let result = (|| {
        let content = fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read graph yaml at {}", yaml_path.display()))?;

        let mut graph: serde_yaml::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse yaml at {}", yaml_path.display()))?;

        // We only care about transforming the `nodes` array
        if let Some(nodes) = graph.get_mut("nodes").and_then(|n| n.as_sequence_mut()) {
            for node in nodes {
                // Check if this node has a `path` property
                if let Some(path_val) = node.get("path").and_then(|p| p.as_str()) {
                    // If the path matches an installed node ID in dm cache
                    let node_cache_dir = node_dir(home, path_val);
                    let meta_file_path = meta_path(home, path_val);

                    if node_cache_dir.exists() && meta_file_path.exists() {
                        let meta_content = fs::read_to_string(&meta_file_path).unwrap_or_default();
                        if let Ok(meta) = serde_json::from_str::<NodeMetaFile>(&meta_content) {
                            // It's a managed package! Transpile it!

                            // 1. Remove the shorthand `path`
                            let node_map = node.as_mapping_mut().unwrap();
                            node_map.remove(serde_yaml::Value::String("path".to_string()));

                            // 2. Build the `custom` execution block based on build strategy
                            let custom_map = build_custom_block(&meta, &node_cache_dir)?;
                            node_map.insert(
                                serde_yaml::Value::String("custom".to_string()),
                                serde_yaml::Value::Mapping(custom_map),
                            );

                            // 3. Build the `env` environment block
                            let env_map = build_env_block(&meta, &node_cache_dir);
                            node_map.insert(
                                serde_yaml::Value::String("env".to_string()),
                                serde_yaml::Value::Mapping(env_map),
                            );
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

fn build_custom_block(meta: &NodeMetaFile, node_dir: &Path) -> Result<serde_yaml::Mapping> {
    let mut custom = serde_yaml::Mapping::new();
    
    // According to dora-message 0.7.0 `CustomNode` and `NodeSource` schema:
    // pub struct CustomNode { pub path: String, pub source: NodeSource, pub args: Option<String> }
    // NodeSource has `Local` and `GitBranch`.
    // So `source` should just be "Local", and `path` should contain the absolute executable path.
    
    custom.insert(
        serde_yaml::Value::String("source".to_string()),
        serde_yaml::Value::String("Local".to_string()),
    );
    
    let build_type = meta.source.build.trim().to_lowercase();
    
    if build_type.starts_with("pip") || build_type.starts_with("uv") {
        // Python node: run via python -m dora_id.main
        let venv_python = node_dir.join(".venv").join("bin").join("python");
        custom.insert(
            serde_yaml::Value::String("path".to_string()),
            serde_yaml::Value::String(venv_python.display().to_string()),
        );

        let module_name = meta.id.replace("-", "_");
        custom.insert(
            serde_yaml::Value::String("args".to_string()),
            serde_yaml::Value::String(format!("-m {}.main", module_name)),
        );
    } else if build_type.starts_with("cargo") {
        // Rust node: run the exact executable
        let bin_name = if meta.id.starts_with("dora-") { meta.id.clone() } else { format!("dora-{}", meta.id) };
        let bin_path = node_dir.join("bin").join(&bin_name);
        
        custom.insert(
            serde_yaml::Value::String("path".to_string()),
            serde_yaml::Value::String(bin_path.display().to_string()),
        );
    } else {
        anyhow::bail!("Cannot transpile custom block for unknown build type: {}", meta.source.build);
    }

    Ok(custom)
}

fn build_env_block(meta: &NodeMetaFile, node_dir: &Path) -> serde_yaml::Mapping {
    let mut env = serde_yaml::Mapping::new();
    let build_type = meta.source.build.trim().to_lowercase();

    if build_type.starts_with("pip") || build_type.starts_with("uv") {
        // For Python, inject the venv virtual environment paths
        let venv_bin = node_dir.join(".venv").join("bin");
        env.insert(
            serde_yaml::Value::String("PATH".to_string()),
            serde_yaml::Value::String(format!("{}:$PATH", venv_bin.display())),
        );
        let venv_site = node_dir.join(".venv").join("lib").join("python3.12").join("site-packages"); // This might need logic to discover the exact python version
        if venv_site.exists() {
             env.insert(
                serde_yaml::Value::String("PYTHONPATH".to_string()),
                serde_yaml::Value::String(venv_site.display().to_string()),
            );
        }
    }
    
    env
}
