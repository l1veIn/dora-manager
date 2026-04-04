use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::events::{EventSource, OperationEvent};
use crate::node::{resolve_dm_json_path, resolve_node_dir, Node};

use super::import;
use super::inspect;
use super::model::{DataflowHistoryEntry, FlowMeta};
use super::repo;
use super::{
    AggregatedConfigField, AggregatedConfigNode, DataflowConfigAggregation,
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
        let view = repo::read_view(home, name).ok();
        Ok(DataflowProject {
            name: name.to_string(),
            yaml,
            meta,
            executable,
            view,
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


pub fn get_flow_meta(home: &Path, name: &str) -> Result<FlowMeta> {
    repo::read_meta(home, name)
}

pub fn save_flow_meta(home: &Path, name: &str, meta: &FlowMeta) -> Result<()> {
    repo::write_meta(home, name, meta)
}

pub fn get_flow_view(home: &Path, name: &str) -> Result<serde_json::Value> {
    repo::read_view(home, name)
}

pub fn save_flow_view(home: &Path, name: &str, view: &serde_json::Value) -> Result<()> {
    repo::write_view(home, name, view)
}

pub fn inspect_config(home: &Path, name: &str) -> Result<DataflowConfigAggregation> {
    let yaml = repo::read_yaml(home, name)?;
    let executable = inspect::inspect_yaml(home, &yaml);
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

            let resolved = resolve_node_dir(home, node_id).is_some();
            let inline_config = entry
                .get("config")
                .and_then(|value| serde_json::to_value(value).ok())
                .unwrap_or_else(|| serde_json::json!({}));

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
            if let Some(schema_obj) = meta
                .config_schema
                .as_ref()
                .and_then(|value| value.as_object())
            {
                for (field_name, field_schema) in schema_obj {
                    let inline_value = inline_config.get(field_name).cloned();
                    let node_value = node_config.get(field_name).cloned();
                    let default_value = field_schema.get("default").cloned();
                    let (effective_value, effective_source) =
                        if let Some(value) = inline_value.clone() {
                            (Some(value), "inline".to_string())
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
            let abs_path = source_path.to_path_buf();
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
