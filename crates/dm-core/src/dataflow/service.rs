use std::path::Path;
use std::collections::BTreeMap;

use anyhow::{Context, Result};

use crate::events::{EventSource, OperationEvent};
use crate::node::{resolve_dm_json_path, resolve_node_dir, Node};

use super::import;
use super::inspect;
use super::model::{DataflowHistoryEntry, FlowMeta};
use super::repo;
use super::{
    AggregatedConfigField, AggregatedConfigNode, DataflowConfigAggregation, DataflowConfigDocument,
    DataflowImportFailure, DataflowImportReport, DataflowImportSuccess, DataflowListEntry,
    DataflowProject,
};

pub fn list(home: &Path) -> Result<Vec<DataflowListEntry>> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.list");
    op.emit_start();
    let result = (|| {
        let mut entries = Vec::new();
        for file in repo::list_projects(home)? {
            let meta = repo::read_meta(home, &file.name).unwrap_or_else(|_| FlowMeta {
                id: file.name.clone(),
                name: file.name.clone(),
                ..Default::default()
            });
            let executable = inspect::inspect(home, &file.name)?.summary;
            entries.push(DataflowListEntry {
                file,
                meta,
                executable,
            });
        }
        Ok(entries)
    })();
    op.emit_result(&result);
    result
}

pub fn get(home: &Path, name: &str) -> Result<DataflowProject> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.get").attr("name", name);
    op.emit_start();
    let result = (|| {
        let yaml = repo::read_yaml(home, name)?;
        let meta = repo::read_meta(home, name).unwrap_or_else(|_| FlowMeta {
            id: name.to_string(),
            name: name.to_string(),
            ..Default::default()
        });
        let executable = inspect::inspect_yaml(home, &yaml).summary;
        Ok(DataflowProject {
            name: name.to_string(),
            yaml,
            meta,
            executable,
        })
    })();
    op.emit_result(&result);
    result
}

pub fn save(home: &Path, name: &str, yaml: &str) -> Result<DataflowProject> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.save").attr("name", name);
    op.emit_start();
    let result = (|| {
        repo::write_yaml(home, name, yaml)?;
        get(home, name)
    })();
    op.emit_result(&result);
    result
}

pub fn delete(home: &Path, name: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.delete").attr("name", name);
    op.emit_start();
    let result = repo::delete_project(home, name);
    op.emit_result(&result);
    result
}

pub fn get_flow_config(home: &Path, name: &str) -> Result<DataflowConfigDocument> {
    let config = repo::read_config(home, name)?;
    let executable = inspect::inspect(home, name)?.summary;
    Ok(DataflowConfigDocument { config, executable })
}

pub fn save_flow_config(
    home: &Path,
    name: &str,
    config: &serde_json::Value,
) -> Result<DataflowConfigDocument> {
    repo::write_config(home, name, config)?;
    get_flow_config(home, name)
}

pub fn get_flow_meta(home: &Path, name: &str) -> Result<FlowMeta> {
    repo::read_meta(home, name)
}

pub fn save_flow_meta(home: &Path, name: &str, meta: &FlowMeta) -> Result<()> {
    repo::write_meta(home, name, meta)
}

pub fn inspect_config(home: &Path, name: &str) -> Result<DataflowConfigAggregation> {
    let yaml = repo::read_yaml(home, name)?;
    let executable = inspect::inspect_yaml(home, &yaml);
    let flow_config = repo::read_config(home, name).unwrap_or_else(|_| serde_json::json!({}));
    let graph: serde_yaml::Value = match serde_yaml::from_str(&yaml) {
        Ok(graph) => graph,
        Err(_) => {
            return Ok(DataflowConfigAggregation {
                executable: executable.summary,
                nodes: Vec::new(),
            })
        }
    };

    let mut nodes = Vec::new();
    if let Some(entries) = graph.get("nodes").and_then(|value| value.as_sequence()) {
        for entry in entries {
            let yaml_id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let Some(node_id) = entry.get("node").and_then(|value| value.as_str()) else {
                continue;
            };
            if node_id == "dm-panel" {
                continue;
            }

            let resolved = resolve_node_dir(home, node_id).is_some();
            let inline_config = entry
                .get("config")
                .and_then(|value| serde_json::to_value(value).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            let flow_node_config = repo::select_flow_node_config(&flow_config, &yaml_id, node_id);

            let Some(node_dir) = resolve_node_dir(home, node_id) else {
                nodes.push(AggregatedConfigNode {
                    yaml_id,
                    node_id: node_id.to_string(),
                    resolved: false,
                    configurable: false,
                    missing_reason: Some("Node is not installed or resolvable".to_string()),
                    fields: BTreeMap::new(),
                });
                continue;
            };

            let meta_path = resolve_dm_json_path(home, node_id);
            let meta = meta_path
                .and_then(|path| std::fs::read_to_string(path).ok())
                .and_then(|content| serde_json::from_str::<Node>(&content).ok());

            let Some(meta) = meta else {
                nodes.push(AggregatedConfigNode {
                    yaml_id,
                    node_id: node_id.to_string(),
                    resolved,
                    configurable: false,
                    missing_reason: Some("Node metadata is unavailable".to_string()),
                    fields: BTreeMap::new(),
                });
                continue;
            };

            let node_config = crate::node::get_node_config(home, node_id)
                .unwrap_or_else(|_| serde_json::json!({}));
            let mut fields = BTreeMap::new();
            if let Some(schema_obj) = meta.config_schema.as_ref().and_then(|value| value.as_object()) {
                for (field_name, field_schema) in schema_obj {
                    let inline_value = inline_config.get(field_name).cloned();
                    let flow_value = flow_node_config.get(field_name).cloned();
                    let node_value = node_config.get(field_name).cloned();
                    let default_value = field_schema.get("default").cloned();
                    let (effective_value, effective_source) = if let Some(value) = inline_value.clone() {
                        (Some(value), "inline".to_string())
                    } else if let Some(value) = flow_value.clone() {
                        (Some(value), "flow".to_string())
                    } else if let Some(value) = node_value.clone() {
                        (Some(value), "node".to_string())
                    } else if let Some(value) = default_value.clone() {
                        (Some(value), "default".to_string())
                    } else {
                        (None, "unset".to_string())
                    };

                    fields.insert(
                        field_name.clone(),
                        AggregatedConfigField {
                            schema: field_schema.clone(),
                            inline_value,
                            flow_value,
                            node_value,
                            default_value,
                            effective_value,
                            effective_source,
                        },
                    );
                }
            }

            nodes.push(AggregatedConfigNode {
                yaml_id,
                node_id: node_id.to_string(),
                resolved: true,
                configurable: true,
                missing_reason: None,
                fields,
            });

            let _ = node_dir;
        }
    }

    Ok(DataflowConfigAggregation {
        executable: executable.summary,
        nodes,
    })
}

pub fn list_history(home: &Path, name: &str) -> Result<Vec<DataflowHistoryEntry>> {
    repo::list_history_versions(home, name)
}

pub fn get_history_version(home: &Path, name: &str, version: &str) -> Result<String> {
    repo::read_history_version(home, name, version)
}

pub fn restore_history_version(home: &Path, name: &str, version: &str) -> Result<()> {
    repo::restore_history_version(home, name, version)
}

pub fn migrate_legacy_layout(home: &Path) -> Result<usize> {
    repo::migrate_legacy_layout(home)
}

pub fn import_local(home: &Path, name: &str, source: &Path) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.import_local")
        .attr("name", name)
        .attr("source", source.display().to_string());
    op.emit_start();
    let result = import::import_local(home, name, source);
    op.emit_result(&result);
    result
}

pub async fn import_git(home: &Path, name: &str, git_url: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "dataflow.import_git")
        .attr("name", name)
        .attr("url", git_url);
    op.emit_start();
    let result = import::import_git(home, name, git_url).await;
    op.emit_result(&result);
    result
}

pub async fn import_sources(home: &Path, sources: &[String]) -> DataflowImportReport {
    let mut imported = Vec::new();
    let mut failed = Vec::new();

    for source in sources {
        let inferred_name = super::infer_import_name(source);
        let is_url = source.starts_with("https://") || source.starts_with("http://");
        let result = if is_url {
            import_git(home, &inferred_name, source).await
        } else {
            let source_path = Path::new(source);
            let abs_path = if source_path.is_absolute() {
                source_path.to_path_buf()
            } else {
                source_path.to_path_buf()
            };
            import_local(home, &inferred_name, &abs_path)
        };

        match result {
            Ok(()) => {
                let executable = match inspect::inspect(home, &inferred_name) {
                    Ok(detail) => detail.summary,
                    Err(err) => super::DataflowExecutableSummary {
                        status: super::DataflowExecutableStatus::InvalidYaml,
                        can_run: false,
                        can_configure: false,
                        declared_node_count: 0,
                        resolved_node_count: 0,
                        missing_node_count: 0,
                        missing_nodes: Vec::new(),
                        invalid_yaml: true,
                        error: Some(err.to_string()),
                    },
                };
                imported.push(DataflowImportSuccess {
                    name: inferred_name,
                    executable,
                });
            }
            Err(err) => failed.push(DataflowImportFailure {
                source: source.clone(),
                name: inferred_name,
                error: err.to_string(),
            }),
        }
    }

    DataflowImportReport { imported, failed }
}

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
        let content = std::fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read graph yaml at {}", yaml_path.display()))?;
        let mut graph: serde_yaml::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse yaml at {}", yaml_path.display()))?;
        let flow_config =
            repo::load_flow_config_for_yaml(home, yaml_path).unwrap_or(serde_json::json!({}));

        if let Some(nodes) = graph.get_mut("nodes").and_then(|n| n.as_sequence_mut()) {
            for node in nodes {
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
                    let dm_exe = std::env::current_exe()
                        .ok()
                        .and_then(|exe| {
                            let dir = exe.parent()?;
                            let dm_path = dir.join("dm");
                            if dm_path.exists() { Some(dm_path) } else { None }
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
                    let meta_content = std::fs::read_to_string(&meta_file_path).unwrap_or_default();
                    if let Ok(meta) = serde_json::from_str::<Node>(&meta_content) {
                        if from_node_field {
                            node_map.remove(serde_yaml::Value::String("node".to_string()));
                        } else {
                            node_map.remove(serde_yaml::Value::String("path".to_string()));
                        }

                        if !meta.executable.is_empty() {
                            let abs_exec = node_cache_dir.join(&meta.executable);
                            node_map.insert(
                                serde_yaml::Value::String("path".to_string()),
                                serde_yaml::Value::String(abs_exec.display().to_string()),
                            );
                        }

                        if let Some(schema) = &meta.config_schema {
                            if let Some(schema_obj) = schema.as_object() {
                                let config_defaults = crate::node::get_node_config(home, &node_id)
                                    .unwrap_or(serde_json::json!({}));
                                let yaml_node_id = node_map
                                    .get(serde_yaml::Value::String("id".to_string()))
                                    .and_then(|value| value.as_str())
                                    .unwrap_or(&node_id);
                                let flow_config_for_node =
                                    repo::select_flow_node_config(&flow_config, yaml_node_id, &node_id);
                                let inline_config = node_map
                                    .get(serde_yaml::Value::String("config".to_string()))
                                    .and_then(|v| serde_json::to_value(v).ok())
                                    .unwrap_or(serde_json::json!({}));

                                let mut env_map = node_map
                                    .get(serde_yaml::Value::String("env".to_string()))
                                    .and_then(|v| v.as_mapping().cloned())
                                    .unwrap_or_default();

                                for (key, field_schema) in schema_obj {
                                    if let Some(env_name) =
                                        field_schema.get("env").and_then(|e| e.as_str())
                                    {
                                        let value = inline_config
                                            .get(key)
                                            .or_else(|| flow_config_for_node.get(key))
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
