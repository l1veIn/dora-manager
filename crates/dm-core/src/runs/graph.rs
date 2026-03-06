use std::collections::BTreeMap;

use anyhow::{Context, Result};

use super::RunTranspileMetadata;

pub(crate) fn extract_node_ids_from_yaml(yaml: &str) -> Result<Vec<String>> {
    let graph: serde_yaml::Value =
        serde_yaml::from_str(yaml).context("Failed to parse dataflow yaml for node inventory")?;
    Ok(extract_node_ids(&graph))
}

pub(crate) fn build_transpile_metadata(graph: &serde_yaml::Value) -> RunTranspileMetadata {
    let mut panel_node_ids = Vec::new();
    let mut resolved_node_paths = BTreeMap::new();
    let working_dir = std::env::current_dir()
        .ok()
        .map(|path| path.display().to_string());

    if let Some(nodes) = graph.get("nodes").and_then(|value| value.as_sequence()) {
        for node in nodes {
            let Some(map) = node.as_mapping() else {
                continue;
            };

            let node_id = map
                .get(serde_yaml::Value::String("id".to_string()))
                .and_then(|value| value.as_str())
                .map(str::to_string);

            if let (Some(id), Some(path)) = (
                node_id.clone(),
                map.get(serde_yaml::Value::String("path".to_string()))
                    .and_then(|value| value.as_str()),
            ) {
                resolved_node_paths.insert(id, path.to_string());
            }

            let is_panel = map
                .get(serde_yaml::Value::String("args".to_string()))
                .and_then(|value| value.as_str())
                .map(|args| args.contains("panel serve --run-id"))
                .unwrap_or(false);

            if is_panel {
                if let Some(id) = node_id {
                    panel_node_ids.push(id);
                }
            }
        }
    }

    panel_node_ids.sort();
    panel_node_ids.dedup();

    RunTranspileMetadata {
        working_dir,
        panel_node_ids,
        resolved_node_paths,
    }
}

fn extract_node_ids(graph: &serde_yaml::Value) -> Vec<String> {
    let Some(nodes) = graph.get("nodes").and_then(|value| value.as_sequence()) else {
        return Vec::new();
    };

    let mut ids = Vec::new();
    for node in nodes {
        let Some(map) = node.as_mapping() else {
            continue;
        };
        let id = map
            .get(serde_yaml::Value::String("id".to_string()))
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| {
                map.get(serde_yaml::Value::String("node".to_string()))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
            .or_else(|| {
                map.get(serde_yaml::Value::String("path".to_string()))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            });

        if let Some(id) = id {
            ids.push(id);
        }
    }

    ids.sort();
    ids.dedup();
    ids
}
