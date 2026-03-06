use std::fs;
use std::path::Path;

use crate::events::{EventSource, OperationEvent};
use crate::node::{resolve_dm_json_path, resolve_node_dir, Node};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
                        let filename = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
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
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.get").attr("name", name);
    op.emit_start();

    let result = {
        let path = home.join("dataflows").join(format!("{}.yml", name));
        fs::read_to_string(&path).with_context(|| format!("Failed to read dataflow '{}'", name))
    };

    op.emit_result(&result);
    result
}

/// Save (create or update) a dataflow's YAML content
pub fn save(home: &Path, name: &str, yaml: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.save").attr("name", name);
    op.emit_start();

    let result = (|| {
        let dir = home.join("dataflows");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.yml", name));
        fs::write(&path, yaml).with_context(|| format!("Failed to save dataflow '{}'", name))
    })();

    op.emit_result(&result);
    result
}

/// Delete a dataflow file
pub fn delete(home: &Path, name: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.delete").attr("name", name);
    op.emit_start();

    let result = {
        let path = home.join("dataflows").join(format!("{}.yml", name));
        fs::remove_file(&path).with_context(|| format!("Failed to delete dataflow '{}'", name))
    };

    op.emit_result(&result);
    result
}

// ─── Dataflow Execution ───

/// Transpile a dataflow yaml file, replacing DM node IDs in `path:` fields
/// with absolute paths to the installed node executables.
///
/// Each managed node has a `dm.json` containing an `executable` field
/// (e.g., `.venv/bin/dora-vad`). Transpile resolves this to an absolute path.
///
/// Config merging: config.json defaults ← dataflow `config:` overrides → env injection.
pub fn transpile_graph(home: &Path, yaml_path: &Path) -> Result<serde_yaml::Value> {
    transpile_graph_for_run(home, yaml_path, &uuid::Uuid::new_v4().to_string())
}

pub fn transpile_graph_for_run(
    home: &Path,
    yaml_path: &Path,
    run_id: &str,
) -> Result<serde_yaml::Value> {
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
                // DM format uses `node:` to reference installed node IDs.
                // Transpiler resolves `node: <id>` → `path: /absolute/exec` for dora.
                // Also supports raw `path:` for non-managed nodes (passthrough).
                let (node_id, from_node_field) =
                    if let Some(v) = node.get("node").and_then(|p| p.as_str()) {
                        (v.to_string(), true)
                    } else if let Some(v) = node.get("path").and_then(|p| p.as_str()) {
                        (v.to_string(), false)
                    } else {
                        continue;
                    };

                let node_map = node.as_mapping_mut().unwrap();

                if node_id == "dm-panel" {
                    let yaml_id = node_map
                        .get(serde_yaml::Value::String("id".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("dm-panel")
                        .to_string();
                    if from_node_field {
                        node_map.remove(serde_yaml::Value::String("node".to_string()));
                    } else {
                        node_map.remove(serde_yaml::Value::String("path".to_string()));
                    }
                    // Resolve the `dm` CLI binary (not dm-server or any other binary).
                    // Look in the same directory as the current executable first.
                    let dm_exe = std::env::current_exe()
                        .ok()
                        .and_then(|exe| {
                            let dir = exe.parent()?;
                            let dm_path = dir.join("dm");
                            if dm_path.exists() {
                                Some(dm_path)
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| std::path::PathBuf::from("dm"));
                    node_map.insert(
                        serde_yaml::Value::String("path".to_string()),
                        serde_yaml::Value::String(dm_exe.display().to_string()),
                    );
                    node_map.insert(
                        serde_yaml::Value::String("args".to_string()),
                        serde_yaml::Value::String(format!(
                            "panel serve --run-id {} --node-id {}",
                            run_id, yaml_id
                        )),
                    );
                    continue;
                }

                let Some(node_cache_dir) = resolve_node_dir(home, &node_id) else {
                    continue;
                };
                let Some(meta_file_path) = resolve_dm_json_path(home, &node_id) else {
                    continue;
                };

                if node_cache_dir.exists() && meta_file_path.exists() {
                    let meta_content = fs::read_to_string(&meta_file_path).unwrap_or_default();
                    if let Ok(meta) = serde_json::from_str::<Node>(&meta_content) {
                        // Remove the source field (`node:` or `path:`)
                        if from_node_field {
                            node_map.remove(serde_yaml::Value::String("node".to_string()));
                        } else {
                            node_map.remove(serde_yaml::Value::String("path".to_string()));
                        }

                        // Insert `path:` with absolute executable path for dora
                        if !meta.executable.is_empty() {
                            let abs_exec = node_cache_dir.join(&meta.executable);
                            node_map.insert(
                                serde_yaml::Value::String("path".to_string()),
                                serde_yaml::Value::String(abs_exec.display().to_string()),
                            );
                        }

                        // 2. Merge config → env injection
                        if let Some(schema) = &meta.config_schema {
                            if let Some(schema_obj) = schema.as_object() {
                                // Read config.json defaults
                                let config_defaults = crate::node::get_node_config(home, &node_id)
                                    .unwrap_or(serde_json::json!({}));

                                // Read dataflow-level config overrides
                                let dataflow_config = node_map
                                    .get(serde_yaml::Value::String("config".to_string()))
                                    .and_then(|v| serde_json::to_value(v).ok())
                                    .unwrap_or(serde_json::json!({}));

                                // Merge: defaults ← overrides
                                let mut env_map = node_map
                                    .get(serde_yaml::Value::String("env".to_string()))
                                    .and_then(|v| v.as_mapping().cloned())
                                    .unwrap_or_default();

                                for (key, field_schema) in schema_obj {
                                    if let Some(env_name) =
                                        field_schema.get("env").and_then(|e| e.as_str())
                                    {
                                        // Priority: dataflow config > config.json > schema default
                                        let value = dataflow_config
                                            .get(key)
                                            .or_else(|| config_defaults.get(key))
                                            .or_else(|| field_schema.get("default"));

                                        if let Some(val) = value {
                                            let val_str = match val {
                                                serde_json::Value::String(s) => s.clone(),
                                                other => other.to_string(),
                                            };
                                            env_map.insert(
                                                serde_yaml::Value::String(env_name.to_string()),
                                                serde_yaml::Value::String(val_str),
                                            );
                                        }
                                    }
                                }

                                if !env_map.is_empty() {
                                    node_map.insert(
                                        serde_yaml::Value::String("env".to_string()),
                                        serde_yaml::Value::Mapping(env_map),
                                    );
                                }

                                // Remove `config:` from output (Dora doesn't understand it)
                                node_map.remove(serde_yaml::Value::String("config".to_string()));
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
