use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::events::{EventSource, OperationEvent};

use super::init::{init_dm_json, InitHints};
use super::model::Node;
use super::paths::{
    configured_node_dirs, dm_json_path, node_dir, resolve_dm_json_path, resolve_node_dir,
};

pub fn create_node(home: &Path, id: &str, description: &str) -> Result<Node> {
    let op = OperationEvent::new(home, EventSource::Core, "node.create").attr("node_id", id);
    op.emit_start();

    let result = (|| {
        let node_path = node_dir(home, id);
        if node_path.exists() {
            bail!("Node '{}' already exists at {}", id, node_path.display());
        }

        let module_name = id.replace('-', "_");
        let module_dir = node_path.join(&module_name);
        std::fs::create_dir_all(&module_dir).with_context(|| {
            format!(
                "Failed to create module directory: {}",
                module_dir.display()
            )
        })?;

        let pyproject = format!(
            r#"[project]
name = "{id}"
version = "0.1.0"
description = "{description}"
requires-python = ">=3.10"
dependencies = ["dora-rs >= 0.3.9", "pyarrow"]

[project.scripts]
{id} = "{module_name}.main:main"
"#,
        );
        std::fs::write(node_path.join("pyproject.toml"), &pyproject)
            .context("Failed to write pyproject.toml")?;

        let main_py = r#"import pyarrow as pa
from dora import Node


def main():
    node = Node()
    for event in node:
        if event["type"] == "INPUT":
            input_id = event["id"]
            value = event["value"]
            # TODO: Process input and send output
            node.send_output("output", pa.array(["processed"]))


if __name__ == "__main__":
    main()
"#;
        std::fs::write(module_dir.join("main.py"), main_py).context("Failed to write main.py")?;
        std::fs::write(module_dir.join("__init__.py"), "")
            .context("Failed to write __init__.py")?;

        let readme = format!(
            "# {id}\n\n{description}\n\n## Usage\n\n```yaml\n- id: {id}\n  path: {id}\n  inputs:\n    input: source/output\n  outputs:\n    - output\n```\n",
        );
        std::fs::write(node_path.join("README.md"), &readme)
            .context("Failed to write README.md")?;

        // Scaffold files written → init_dm_json will read pyproject.toml to infer metadata
        init_dm_json(
            id,
            &node_path,
            InitHints {
                description: Some(description.to_string()),
            },
        )
    })();

    op.emit_result(&result);
    result
}

pub fn list_nodes(home: &Path) -> Result<Vec<Node>> {
    let op = OperationEvent::new(home, EventSource::Core, "node.list");
    op.emit_start();

    let result = (|| {
        let mut nodes = Vec::new();
        let mut seen = std::collections::BTreeSet::new();

        for nodes_path in configured_node_dirs(home) {
            if !nodes_path.exists() {
                continue;
            }

            for entry in std::fs::read_dir(&nodes_path)
                .with_context(|| format!("Failed to read directory: {}", nodes_path.display()))?
            {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let id = match entry.file_name().to_str() {
                    Some(name) => name.to_string(),
                    None => continue,
                };
                if !seen.insert(id.clone()) {
                    continue;
                }

                let meta_file = path.join("dm.json");
                if let Ok(content) = std::fs::read_to_string(&meta_file) {
                    if let Ok(node) = serde_json::from_str::<Node>(&content) {
                        nodes.push(node.with_path(path));
                        continue;
                    }
                }

                nodes.push(Node::fallback(id, path));
            }
        }

        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(nodes)
    })();

    op.emit_result(&result);
    result
}

pub fn uninstall_node(home: &Path, id: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "node.uninstall").attr("node_id", id);
    op.emit_start();

    let result = (|| {
        let node_path = node_dir(home, id);
        if node_path.exists() {
            std::fs::remove_dir_all(&node_path).with_context(|| {
                format!("Failed to remove node directory: {}", node_path.display())
            })?;
            return Ok(());
        }

        if resolve_node_dir(home, id).is_some() {
            bail!(
                "Node '{}' is builtin and cannot be uninstalled from the managed node directory",
                id
            );
        }

        bail!("Node '{}' is not installed", id);
    })();

    op.emit_result(&result);
    result
}

pub fn get_node_readme(home: &Path, id: &str) -> Result<String> {
    let readme_path = resolve_node_dir(home, id)
        .ok_or_else(|| anyhow::anyhow!("Node '{}' does not exist", id))?
        .join("README.md");
    std::fs::read_to_string(&readme_path)
        .with_context(|| format!("Failed to read README for node '{}'", id))
}

pub fn get_node_config(home: &Path, id: &str) -> Result<serde_json::Value> {
    let Some(node_path) = resolve_node_dir(home, id) else {
        return Ok(serde_json::json!({}));
    };
    let config_path = node_path.join("config.json");
    if !config_path.exists() {
        return Ok(serde_json::json!({}));
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config for node '{}'", id))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config.json for node '{}'", id))
}

pub fn save_node_config(home: &Path, id: &str, config: &serde_json::Value) -> Result<()> {
    let node_path = resolve_node_dir(home, id)
        .ok_or_else(|| anyhow::anyhow!("Node '{}' does not exist", id))?;
    if !node_path.exists() {
        bail!("Node '{}' does not exist", id);
    }

    let config_json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    std::fs::write(node_path.join("config.json"), config_json)
        .with_context(|| format!("Failed to write config.json for node '{}'", id))
}

pub fn node_status(home: &Path, id: &str) -> Result<Option<Node>> {
    let op = OperationEvent::new(home, EventSource::Core, "node.status").attr("node_id", id);
    op.emit_start();

    let result = (|| {
        let Some(node_path) = resolve_node_dir(home, id) else {
            return Ok(None);
        };
        if !node_path.exists() {
            return Ok(None);
        }

        let meta_file = resolve_dm_json_path(home, id).unwrap_or_else(|| dm_json_path(home, id));
        match std::fs::read_to_string(&meta_file) {
            Ok(content) => {
                let node: Node =
                    serde_json::from_str(&content).context("Failed to parse node metadata")?;
                Ok(Some(node.with_path(node_path)))
            }
            Err(_) => Ok(Some(Node::fallback(id.to_string(), node_path))),
        }
    })();

    op.emit_result(&result);
    result
}
