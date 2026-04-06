use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;

use crate::node::{resolve_dm_json_path, resolve_node_dir, Node};

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
                requires_media_backend: false,
                media_node_count: 0,
                media_nodes: Vec::new(),
                error: Some(err.to_string()),
            },
            nodes: Vec::new(),
        },
    }
}

fn inspect_graph(home: &Path, graph: &serde_yaml::Value) -> DataflowExecutableDetail {
    let mut nodes = Vec::new();
    let mut missing_nodes = BTreeSet::new();
    let mut media_nodes = BTreeSet::new();
    let mut resolved_node_count = 0usize;

    if let Some(entries) = graph.get("nodes").and_then(|n| n.as_sequence()) {
        for entry in entries {
            let yaml_id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            if let Some(node_id) = entry.get("node").and_then(|value| value.as_str()) {
                let resolved = resolve_node_dir(home, node_id).is_some();
                let configurable = resolved && resolve_dm_json_path(home, node_id).is_some();
                if resolved && node_requires_media_backend(home, node_id) {
                    media_nodes.insert(node_id.to_string());
                }
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
    let media_nodes: Vec<String> = media_nodes.into_iter().collect();
    let media_node_count = media_nodes.len();

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
            requires_media_backend: media_node_count > 0,
            media_node_count,
            media_nodes,
            error: None,
        },
        nodes,
    }
}

fn node_requires_media_backend(home: &Path, node_id: &str) -> bool {
    let Some(path) = resolve_dm_json_path(home, node_id) else {
        return false;
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(node) = serde_json::from_str::<Node>(&content) else {
        return false;
    };
    node.capabilities
        .iter()
        .any(|capability| capability == "media")
}
