use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::events::{EventSource, OperationEvent};

use super::model::{fallback_entry, NodeMetaFile};
use super::paths::{dm_json_path, node_dir, nodes_dir};
use super::{current_timestamp, NodeEntry, NodeSource};

pub fn create_node(home: &Path, id: &str, description: &str) -> Result<NodeEntry> {
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

        let dm_meta = NodeMetaFile {
            id: id.to_string(),
            name: id.to_string(),
            version: "0.1.0".to_string(),
            installed_at: current_timestamp(),
            source: NodeSource {
                build: "pip install -e .".to_string(),
                github: None,
            },
            description: description.to_string(),
            executable: String::new(),
            author: None,
            category: String::new(),
            inputs: Vec::new(),
            outputs: vec!["output".to_string()],
            avatar: None,
            config_schema: None,
        };

        let dm_path = dm_json_path(home, id);
        let dm_json =
            serde_json::to_string_pretty(&dm_meta).context("Failed to serialize dm.json")?;
        std::fs::write(&dm_path, dm_json)
            .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

        Ok(dm_meta.into_entry(node_path))
    })();

    op.emit_result(&result);
    result
}

pub fn list_nodes(home: &Path) -> Result<Vec<NodeEntry>> {
    let op = OperationEvent::new(home, EventSource::Core, "node.list");
    op.emit_start();

    let result = (|| {
        let nodes_path = nodes_dir(home);
        if !nodes_path.exists() {
            return Ok(Vec::new());
        }

        let mut nodes = Vec::new();
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

            let meta_file = dm_json_path(home, &id);
            if let Ok(content) = std::fs::read_to_string(&meta_file) {
                if let Ok(meta) = serde_json::from_str::<NodeMetaFile>(&content) {
                    nodes.push(meta.into_entry(path));
                    continue;
                }
            }

            nodes.push(fallback_entry(id, path));
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
        if !node_path.exists() {
            bail!("Node '{}' is not installed", id);
        }

        std::fs::remove_dir_all(&node_path)
            .with_context(|| format!("Failed to remove node directory: {}", node_path.display()))?;
        Ok(())
    })();

    op.emit_result(&result);
    result
}

pub fn get_node_readme(home: &Path, id: &str) -> Result<String> {
    let readme_path = node_dir(home, id).join("README.md");
    std::fs::read_to_string(&readme_path)
        .with_context(|| format!("Failed to read README for node '{}'", id))
}

pub fn get_node_config(home: &Path, id: &str) -> Result<serde_json::Value> {
    let config_path = node_dir(home, id).join("config.json");
    if !config_path.exists() {
        return Ok(serde_json::json!({}));
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config for node '{}'", id))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config.json for node '{}'", id))
}

pub fn save_node_config(home: &Path, id: &str, config: &serde_json::Value) -> Result<()> {
    let node_path = node_dir(home, id);
    if !node_path.exists() {
        bail!("Node '{}' does not exist", id);
    }

    let config_json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    std::fs::write(node_path.join("config.json"), config_json)
        .with_context(|| format!("Failed to write config.json for node '{}'", id))
}

pub fn node_status(home: &Path, id: &str) -> Result<Option<NodeEntry>> {
    let op = OperationEvent::new(home, EventSource::Core, "node.status").attr("node_id", id);
    op.emit_start();

    let result = (|| {
        let node_path = node_dir(home, id);
        if !node_path.exists() {
            return Ok(None);
        }

        let meta_file = dm_json_path(home, id);
        match std::fs::read_to_string(&meta_file) {
            Ok(content) => {
                let meta: NodeMetaFile =
                    serde_json::from_str(&content).context("Failed to parse node metadata")?;
                Ok(Some(meta.into_entry(node_path)))
            }
            Err(_) => Ok(Some(fallback_entry(id.to_string(), node_path))),
        }
    })();

    op.emit_result(&result);
    result
}
