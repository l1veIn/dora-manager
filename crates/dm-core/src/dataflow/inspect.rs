use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;

use crate::node::{resolve_dm_json_path, resolve_node_dir};

use super::model::{
    DataflowExecutableDetail, DataflowExecutableStatus, DataflowExecutableSummary,
    DataflowNodeResolution,
};
use super::repo::read_yaml;

pub fn inspect(home: &Path, name: &str) -> Result<DataflowExecutableDetail> {
    let yaml = read_yaml(home, name)?;
    Ok(inspect_yaml(home, &yaml))
}

pub fn inspect_yaml(home: &Path, yaml: &str) -> DataflowExecutableDetail {
    match serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        Ok(graph) => inspect_graph(home, &graph),
        Err(err) => DataflowExecutableDetail {
            summary: DataflowExecutableSummary {
                status: DataflowExecutableStatus::InvalidYaml,
                can_run: false,
                can_configure: false,
                declared_node_count: 0,
                resolved_node_count: 0,
                missing_node_count: 0,
                missing_nodes: Vec::new(),
                invalid_yaml: true,
                error: Some(err.to_string()),
            },
            nodes: Vec::new(),
        },
    }
}

fn inspect_graph(home: &Path, graph: &serde_yaml::Value) -> DataflowExecutableDetail {
    let mut nodes = Vec::new();
    let mut missing_nodes = BTreeSet::new();
    let mut resolved_node_count = 0usize;

    if let Some(entries) = graph.get("nodes").and_then(|n| n.as_sequence()) {
        for entry in entries {
            let yaml_id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            if let Some(node_id) = entry.get("node").and_then(|value| value.as_str()) {
                // Builtin reserved nodes (compiled into the dm binary)
                if node_id == "dm-panel" || node_id == "dm-test-harness" {
                    resolved_node_count += 1;
                    nodes.push(DataflowNodeResolution {
                        yaml_id,
                        node_id: node_id.to_string(),
                        resolved: true,
                        configurable: false,
                        source: "builtin".to_string(),
                    });
                    continue;
                }

                let resolved = resolve_node_dir(home, node_id).is_some();
                let configurable = resolved && resolve_dm_json_path(home, node_id).is_some();
                if resolved {
                    resolved_node_count += 1;
                } else {
                    missing_nodes.insert(node_id.to_string());
                }
                nodes.push(DataflowNodeResolution {
                    yaml_id,
                    node_id: node_id.to_string(),
                    resolved,
                    configurable,
                    source: "managed_node".to_string(),
                });
            } else if let Some(path_value) = entry.get("path").and_then(|value| value.as_str()) {
                nodes.push(DataflowNodeResolution {
                    yaml_id,
                    node_id: path_value.to_string(),
                    resolved: true,
                    configurable: false,
                    source: "external_path".to_string(),
                });
            }
        }
    }

    let missing_nodes: Vec<String> = missing_nodes.into_iter().collect();
    let status = if missing_nodes.is_empty() {
        DataflowExecutableStatus::Ready
    } else {
        DataflowExecutableStatus::MissingNodes
    };
    let can_run = matches!(status, DataflowExecutableStatus::Ready);
    let can_configure = missing_nodes.is_empty();
    let declared_node_count = nodes.len();
    let missing_node_count = missing_nodes.len();

    DataflowExecutableDetail {
        summary: DataflowExecutableSummary {
            status,
            can_run,
            can_configure,
            declared_node_count,
            resolved_node_count,
            missing_node_count,
            missing_nodes,
            invalid_yaml: false,
            error: None,
        },
        nodes,
    }
}
