use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;

use crate::node::{resolve_dm_json_path, resolve_node_dir, Node};
use crate::node::hub;

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
                missing_nodes_with_git_url: None,
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
    let mut missing_nodes_with_git_url = std::collections::BTreeMap::new();
    let mut media_nodes = BTreeSet::new();
    let mut resolved_node_count = 0usize;

    if let Some(entries) = graph.get("nodes").and_then(|n| n.as_sequence()) {
        for entry in entries {
            let yaml_id = entry
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            
            // Check for source.git field
            let source_git_url = entry
                .get("source")
                .and_then(|source| source.as_mapping())
                .and_then(|source_map| source_map.get(serde_yaml::Value::String("git".to_string())))
                .and_then(|git| git.as_str())
                .map(|s| s.to_string());
            
            if let Some(node_id) = entry.get("node").and_then(|value| value.as_str()) {
                let resolved = resolve_node_dir(home, node_id).is_some();
                let configurable = resolved && resolve_dm_json_path(home, node_id).is_some();
                if resolved && node_requires_media_backend(home, node_id) {
                    media_nodes.insert(node_id.to_string());
                }
                if resolved {
                    resolved_node_count += 1;
                } else {
                    // Check if we have a git URL from source.git or registry
                    let git_url = source_git_url.clone()
                        .or_else(|| hub::resolve_node_source(node_id).map(|s| s.to_string()));
                    
                    missing_nodes.insert(node_id.to_string());
                    if let Some(url) = git_url {
                        missing_nodes_with_git_url.insert(node_id.to_string(), url);
                    }
                }
                nodes.push(DataflowNodeResolution {
                    yaml_id,
                    node_id: node_id.to_string(),
                    resolved,
                    configurable,
                    source: "managed_node".to_string(),
                    source_git_url,
                });
            } else if let Some(path_value) = entry.get("path").and_then(|value| value.as_str()) {
                nodes.push(DataflowNodeResolution {
                    yaml_id,
                    node_id: path_value.to_string(),
                    resolved: true,
                    configurable: false,
                    source: "external_path".to_string(),
                    source_git_url: None,
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
            missing_nodes_with_git_url: if missing_nodes_with_git_url.is_empty() {
                None
            } else {
                Some(missing_nodes_with_git_url)
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn inspect_yaml_with_source_git_field() {
        let tmp = tempdir().unwrap();
        let home = tmp.path();

        let yaml = r#"
nodes:
  - id: test-node
    node: missing-node
    source:
      git: https://github.com/example/test-node.git
"#;

        let detail = inspect_yaml(home, yaml);
        assert_eq!(detail.summary.status, DataflowExecutableStatus::MissingNodes);
        assert_eq!(detail.summary.missing_nodes, vec!["missing-node"]);
        assert!(detail.summary.missing_nodes_with_git_url.is_some());
        let git_urls = detail.summary.missing_nodes_with_git_url.unwrap();
        assert_eq!(git_urls.get("missing-node").unwrap(), "https://github.com/example/test-node.git");
    }

    #[test]
    fn inspect_yaml_with_registry_node() {
        let tmp = tempdir().unwrap();
        let home = tmp.path();

        let yaml = r#"
nodes:
  - id: test-node
    node: dora-echo
"#;

        let detail = inspect_yaml(home, yaml);
        assert_eq!(detail.summary.status, DataflowExecutableStatus::MissingNodes);
        assert_eq!(detail.summary.missing_nodes, vec!["dora-echo"]);
        assert!(detail.summary.missing_nodes_with_git_url.is_some());
        let git_urls = detail.summary.missing_nodes_with_git_url.unwrap();
        assert_eq!(git_urls.get("dora-echo").unwrap(), "https://github.com/dora-rs/dora-echo.git");
    }

    #[test]
    fn inspect_yaml_without_git_url() {
        let tmp = tempdir().unwrap();
        let home = tmp.path();

        let yaml = r#"
nodes:
  - id: test-node
    node: unknown-node
"#;

        let detail = inspect_yaml(home, yaml);
        assert_eq!(detail.summary.status, DataflowExecutableStatus::MissingNodes);
        assert_eq!(detail.summary.missing_nodes, vec!["unknown-node"]);
        // Should not have git URL since it's not in registry
        assert!(detail.summary.missing_nodes_with_git_url.is_none());
    }
}
