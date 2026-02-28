//! Node Manager - Install, list, and manage local dora nodes
//!
//! Nodes are installed in `~/.dm/nodes/<id>/` with metadata stored in `dm.json`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::events::{EventSource, OperationEvent};
use crate::registry::{self, NodeMeta};

/// Installed node entry with local metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEntry {
    /// Unique node identifier (e.g., "llama-vision")
    pub id: String,
    /// Human-readable display name
    #[serde(default)]
    pub name: String,
    /// Version of the installed node
    pub version: String,
    /// Path to the node installation directory
    pub path: PathBuf,
    /// Installation timestamp (ISO 8601 format)
    pub installed_at: String,
    /// Short description
    #[serde(default)]
    pub description: String,
    /// Author name
    #[serde(default)]
    pub author: Option<String>,
    /// Category tag (e.g., "vision", "llm")
    #[serde(default)]
    pub category: String,
    /// Declared input ports
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Declared output ports
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Avatar / icon URL
    #[serde(default)]
    pub avatar: Option<String>,
}

/// Metadata file stored in each node's directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetaFile {
    pub id: String,
    /// Human-readable display name
    #[serde(default)]
    pub name: String,
    pub version: String,
    pub installed_at: String,
    pub source: NodeSource,
    /// Short description
    #[serde(default)]
    pub description: String,
    /// Relative path to the node executable (e.g., ".venv/bin/dora-vad" on Unix,
    /// ".venv/Scripts/dora-vad.exe" on Windows, "bin/dora-rerun" for Rust nodes).
    /// Determined at install time; transpile simply resolves this to an absolute path.
    #[serde(default)]
    pub executable: String,
    /// Author name
    #[serde(default)]
    pub author: Option<String>,
    /// Category tag (e.g., "vision", "llm")
    #[serde(default)]
    pub category: String,
    /// Declared input ports
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Declared output ports
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Avatar / icon URL
    #[serde(default)]
    pub avatar: Option<String>,
    /// Configuration schema for node-level settings (e.g., API keys, model params).
    /// Will be used for GUI config panels and transpile-time validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
}

/// Source information for an installed node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSource {
    pub build: String,
    pub github: Option<String>,
}

// ─── Directory helpers ───

/// Get the nodes directory: `~/.dora/nodes/`
fn nodes_dir(home: &Path) -> PathBuf {
    home.join("nodes")
}

/// Get a specific node's directory
pub fn node_dir(home: &Path, id: &str) -> PathBuf {
    nodes_dir(home).join(id)
}

/// Get the dm.json path for a node
pub fn dm_json_path(home: &Path, id: &str) -> PathBuf {
    node_dir(home, id).join("dm.json")
}

/// Get current ISO 8601 timestamp
fn current_timestamp() -> String {
    // Use a simple format without external dependencies
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    // Simple ISO-like format: YYYY-MM-DDTHH:MM:SSZ
    // We'll just use Unix timestamp for simplicity
    format!("{}", secs)
}

// ─── Public API ───

/// Download a node from the registry (without installing)
///
/// This function:
/// 1. Fetches the registry to find the node
/// 2. Creates the node directory
/// 3. Writes dm.json with executable empty (not yet installed)
pub async fn download_node(home: &Path, id: &str) -> Result<NodeEntry> {
    let op = OperationEvent::new(home, EventSource::Core, "node.download").attr("node_id", id);
    op.emit_start();

    let result = async {
        let registry = registry::fetch_registry()
            .await
            .context("Failed to fetch registry")?;

        let meta = registry::find_node(&registry, id)
            .ok_or_else(|| anyhow::anyhow!("Node '{}' not found in registry", id))?;

        let node_path = node_dir(home, id);
        if node_path.exists() {
            bail!("Node '{}' already exists at {}", id, node_path.display());
        }

        std::fs::create_dir_all(&node_path)
            .with_context(|| format!("Failed to create directory: {}", node_path.display()))?;

        // Write dm.json with empty executable (not installed yet)
        let dm_meta = NodeMetaFile {
            id: id.to_string(),
            name: meta.name.clone(),
            version: String::new(), // will be determined at install time
            installed_at: current_timestamp(),
            source: NodeSource {
                build: meta.build.clone(),
                github: None,
            },
            description: meta.description.clone(),
            executable: String::new(), // empty = not installed
            author: None,
            category: meta.category.clone(),
            inputs: meta.inputs.clone(),
            outputs: meta.outputs.clone(),
            avatar: None,
            config_schema: None,
        };

        let dm_path = dm_json_path(home, id);
        let dm_json = serde_json::to_string_pretty(&dm_meta)
            .context("Failed to serialize dm.json")?;
        std::fs::write(&dm_path, dm_json)
            .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

        Ok(NodeEntry {
            id: id.to_string(),
            name: dm_meta.name,
            version: dm_meta.version,
            path: node_path,
            installed_at: dm_meta.installed_at,
            description: dm_meta.description,
            author: dm_meta.author,
            category: dm_meta.category,
            inputs: dm_meta.inputs,
            outputs: dm_meta.outputs,
            avatar: dm_meta.avatar,
        })
    }
    .await;

    op.emit_result(&result);
    result
}

/// Install a node that already exists in the nodes directory
///
/// Creates venv, runs pip/cargo install, updates dm.json with executable path.
/// Can be called after download_node, create_node, or to re-install after changes.
pub async fn install_node(home: &Path, id: &str) -> Result<NodeEntry> {
    let op = OperationEvent::new(home, EventSource::Core, "node.install").attr("node_id", id);
    op.emit_start();

    let result = async {
        let node_path = node_dir(home, id);
        let dm_path = dm_json_path(home, id);

        if !node_path.exists() || !dm_path.exists() {
            bail!("Node '{}' not found. Download or create it first.", id);
        }

        // Read existing dm.json
        let dm_content = std::fs::read_to_string(&dm_path)
            .with_context(|| format!("Failed to read dm.json for '{}'", id))?;
        let mut dm_meta: NodeMetaFile = serde_json::from_str(&dm_content)
            .with_context(|| format!("Failed to parse dm.json for '{}'", id))?;

        let build_type = dm_meta.source.build.trim().to_lowercase();

        // Run the actual installation
        if build_type.starts_with("pip") || build_type.starts_with("uv") {
            // Check if there's a local pyproject.toml (created by create_node)
            let has_local_pyproject = node_path.join("pyproject.toml").exists();

            let registry_meta = registry::NodeMeta {
                id: dm_meta.id.clone(),
                name: dm_meta.name.clone(),
                build: dm_meta.source.build.clone(),
                description: dm_meta.description.clone(),
                category: dm_meta.category.clone(),
                inputs: dm_meta.inputs.clone(),
                outputs: dm_meta.outputs.clone(),
                system_deps: None,
                tags: Vec::new(),
            };

            let version = if has_local_pyproject {
                // Local node: create venv + pip install -e .
                install_local_python_node(&node_path).await?
            } else {
                // Registry node: create venv + pip install <package>
                install_python_node(&registry_meta, &node_path).await?
            };

            dm_meta.version = version;

            // Determine executable path
            dm_meta.executable = if cfg!(windows) {
                format!(".venv/Scripts/{}.exe", id)
            } else {
                format!(".venv/bin/{}", id)
            };
        } else if build_type.starts_with("cargo") {
            let registry_meta = registry::NodeMeta {
                id: dm_meta.id.clone(),
                name: dm_meta.name.clone(),
                build: dm_meta.source.build.clone(),
                description: dm_meta.description.clone(),
                category: dm_meta.category.clone(),
                inputs: dm_meta.inputs.clone(),
                outputs: dm_meta.outputs.clone(),
                system_deps: None,
                tags: Vec::new(),
            };

            let version = install_cargo_node(&registry_meta, &node_path).await?;
            dm_meta.version = version;

            let bin_name = if id.starts_with("dora-") {
                id.to_string()
            } else {
                format!("dora-{}", id)
            };
            dm_meta.executable = if cfg!(windows) {
                format!("bin/{}.exe", bin_name)
            } else {
                format!("bin/{}", bin_name)
            };
        } else {
            bail!("Unsupported build type: '{}'", dm_meta.source.build);
        }

        dm_meta.installed_at = current_timestamp();

        // Write updated dm.json
        let dm_json = serde_json::to_string_pretty(&dm_meta)
            .context("Failed to serialize dm.json")?;
        std::fs::write(&dm_path, dm_json)
            .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

        Ok(NodeEntry {
            id: id.to_string(),
            name: dm_meta.name,
            version: dm_meta.version,
            path: node_path,
            installed_at: dm_meta.installed_at,
            description: dm_meta.description,
            author: dm_meta.author,
            category: dm_meta.category,
            inputs: dm_meta.inputs,
            outputs: dm_meta.outputs,
            avatar: dm_meta.avatar,
        })
    }
    .await;

    op.emit_result(&result);
    result
}


/// Create a new local Python node scaffold
///
/// Generates:
/// - `pyproject.toml` with console_scripts entry
/// - `<module>/main.py` with Dora Node template
/// - `<module>/__init__.py`  
/// - `README.md`
/// - `dm.json` (executable empty = not yet installed)
pub fn create_node(home: &Path, id: &str, description: &str) -> Result<NodeEntry> {
    let op = OperationEvent::new(home, EventSource::Core, "node.create").attr("node_id", id);
    op.emit_start();

    let result = (|| {
        let node_path = node_dir(home, id);
        if node_path.exists() {
            bail!("Node '{}' already exists at {}", id, node_path.display());
        }

        let module_name = id.replace("-", "_");

        // Create directories
        let module_dir = node_path.join(&module_name);
        std::fs::create_dir_all(&module_dir)
            .with_context(|| format!("Failed to create module directory: {}", module_dir.display()))?;

        // Generate pyproject.toml
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
            id = id,
            description = description,
            module_name = module_name,
        );
        std::fs::write(node_path.join("pyproject.toml"), &pyproject)
            .context("Failed to write pyproject.toml")?;

        // Generate main.py template
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
        std::fs::write(module_dir.join("main.py"), main_py)
            .context("Failed to write main.py")?;

        // Generate __init__.py
        std::fs::write(module_dir.join("__init__.py"), "")
            .context("Failed to write __init__.py")?;

        // Generate README.md
        let readme = format!(
            "# {id}\n\n{description}\n\n## Usage\n\n```yaml\n- id: {id}\n  path: {id}\n  inputs:\n    input: source/output\n  outputs:\n    - output\n```\n",
            id = id,
            description = description,
        );
        std::fs::write(node_path.join("README.md"), &readme)
            .context("Failed to write README.md")?;

        // Write dm.json (executable empty = not yet installed)
        let dm_meta = NodeMetaFile {
            id: id.to_string(),
            name: id.to_string(),
            version: "0.1.0".to_string(),
            installed_at: current_timestamp(),
            source: NodeSource {
                build: format!("pip install -e ."),
                github: None,
            },
            description: description.to_string(),
            executable: String::new(), // empty = not installed
            author: None,
            category: String::new(),
            inputs: Vec::new(),
            outputs: vec!["output".to_string()],
            avatar: None,
            config_schema: None,
        };

        let dm_path = dm_json_path(home, id);
        let dm_json = serde_json::to_string_pretty(&dm_meta)
            .context("Failed to serialize dm.json")?;
        std::fs::write(&dm_path, dm_json)
            .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

        Ok(NodeEntry {
            id: id.to_string(),
            name: dm_meta.name,
            version: dm_meta.version,
            path: node_path,
            installed_at: dm_meta.installed_at,
            description: dm_meta.description,
            author: dm_meta.author,
            category: dm_meta.category,
            inputs: dm_meta.inputs,
            outputs: dm_meta.outputs,
            avatar: dm_meta.avatar,
        })
    })();

    op.emit_result(&result);
    result
}

/// Install a local Python node that has a pyproject.toml (editable install)
async fn install_local_python_node(node_path: &Path) -> Result<String> {
    let venv_path = node_path.join(".venv");

    // Create venv (try uv first)
    let use_uv = Command::new("uv")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let venv_result = if use_uv {
        Command::new("uv")
            .args(["venv", &venv_path.to_string_lossy()])
            .status()
    } else {
        Command::new("python3")
            .args(["-m", "venv", &venv_path.to_string_lossy()])
            .status()
    };

    venv_result
        .with_context(|| format!("Failed to create venv at {}", venv_path.display()))?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to create virtual environment"))?;

    // pip install -e . (editable install)
    let install_result = if use_uv {
        Command::new("uv")
            .args(["pip", "install", "--python", &format!("{}/bin/python", venv_path.display()), "-e", "."])
            .current_dir(node_path)
            .status()
    } else {
        Command::new(format!("{}/bin/pip", venv_path.display()))
            .args(["install", "-e", "."])
            .current_dir(node_path)
            .status()
    };

    match install_result {
        Ok(status) if status.success() => Ok("0.1.0".to_string()),
        Ok(_) => bail!("Failed to install local node via pip install -e ."),
        Err(e) => bail!("Failed to run pip install: {}", e),
    }
}


/// Install a Python-based node using uv/pip
async fn install_python_node(meta: &NodeMeta, node_path: &Path) -> Result<String> {
    // Create a virtual environment
    let venv_path = node_path.join(".venv");
    
    // Try uv first, fallback to python -m venv
    let use_uv = Command::new("uv")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let venv_result = if use_uv {
        Command::new("uv")
            .args(["venv", &venv_path.to_string_lossy()])
            .status()
    } else {
        Command::new("python3")
            .args(["-m", "venv", &venv_path.to_string_lossy()])
            .status()
    };

    venv_result
        .with_context(|| format!("Failed to create virtual environment at {}", venv_path.display()))?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to create virtual environment"))?;

    // Install the node package
    // Extract actual package name from `build` command if possible, otherwise fallback
    let package_spec = if meta.build.starts_with("pip install ") {
        meta.build.trim_start_matches("pip install ").trim().to_string()
    } else if meta.id.starts_with("dora-") {
        meta.id.clone()
    } else {
        format!("dora-{}", meta.id)
    };
    
    let install_result = if use_uv {
        Command::new("uv")
            .args(["pip", "install", "--python", &format!("{}/bin/python", venv_path.display()), &package_spec])
            .status()
    } else {
        Command::new(format!("{}/bin/pip", venv_path.display()))
            .args(["install", &package_spec])
            .status()
    };

    match install_result {
        Ok(status) if status.success() => {
            // Get installed version
            let version = get_python_package_version(&venv_path, &package_spec)?;
            Ok(version)
        }
        Ok(_) => bail!("Failed to install package: {}", package_spec),
        Err(e) => bail!("Failed to run pip install: {}", e),
    }
}

/// Get the installed version of a Python package
fn get_python_package_version(venv_path: &Path, package: &str) -> Result<String> {
    let output = Command::new(format!("{}/bin/python", venv_path.display()))
        .args([
            "-c",
            &format!(
                "import importlib.metadata; print(importlib.metadata.version('{}'))",
                package
            ),
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            Ok(if version.is_empty() { "unknown".to_string() } else { version })
        }
        _ => Ok("unknown".to_string()),
    }
}

/// Install a Cargo-based node
async fn install_cargo_node(meta: &NodeMeta, node_path: &Path) -> Result<String> {
    // Check cargo is available
    let cargo_available = Command::new("cargo")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !cargo_available {
        bail!("Cargo is not installed. Please install Rust first.");
    }

    // Build the node using cargo
    let package_name = format!("dora-{}", meta.id);
    
    let status = Command::new("cargo")
        .args(["install", "--root", &node_path.to_string_lossy(), &package_name])
        .status()
        .with_context(|| "Failed to run cargo install")?;

    if !status.success() {
        bail!("Failed to install cargo package: {}", package_name);
    }

    // Get installed version
    let version = get_crate_version(node_path, &package_name).unwrap_or_else(|_| "unknown".to_string());
    Ok(version)
}

/// Get the installed version of a cargo crate
fn get_crate_version(_node_path: &Path, _package: &str) -> Result<String> {
    // Check cargo metadata or the binary version
    // For simplicity, we just return "latest" for now
    // In the future, this could parse Cargo.toml or query the binary
    Ok("latest".to_string())
}

/// List all installed nodes
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

            // Try to read metadata
            let meta_file = dm_json_path(home, &id);
            if let Ok(content) = std::fs::read_to_string(&meta_file) {
                if let Ok(meta) = serde_json::from_str::<NodeMetaFile>(&content) {
                    nodes.push(NodeEntry {
                        id: meta.id,
                        name: meta.name,
                        version: meta.version,
                        path,
                        installed_at: meta.installed_at,
                        description: meta.description,
                        author: meta.author,
                        category: meta.category,
                        inputs: meta.inputs,
                        outputs: meta.outputs,
                        avatar: meta.avatar,
                    });
                    continue;
                }
            }

            // Fallback: create entry from directory info
            nodes.push(NodeEntry {
                id,
                name: String::new(),
                version: "unknown".to_string(),
                path,
                installed_at: "unknown".to_string(),
                description: String::new(),
                author: None,
                category: String::new(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                avatar: None,
            });
        }

        // Sort by id
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(nodes)
    })();

    op.emit_result(&result);
    result
}

/// Uninstall a node
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

/// Get the README content of a node
pub fn get_node_readme(home: &Path, id: &str) -> Result<String> {
    let readme_path = node_dir(home, id).join("README.md");
    std::fs::read_to_string(&readme_path)
        .with_context(|| format!("Failed to read README for node '{}'", id))
}

/// Get the config values for a node (from config.json)
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

/// Save config values for a node (to config.json)
pub fn save_node_config(home: &Path, id: &str, config: &serde_json::Value) -> Result<()> {
    let node_path = node_dir(home, id);
    if !node_path.exists() {
        bail!("Node '{}' does not exist", id);
    }
    let config_json = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;
    std::fs::write(node_path.join("config.json"), config_json)
        .with_context(|| format!("Failed to write config.json for node '{}'", id))
}

/// Get the status of a specific node
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
                let meta: NodeMetaFile = serde_json::from_str(&content)
                    .context("Failed to parse node metadata")?;
                Ok(Some(NodeEntry {
                    id: meta.id,
                    name: meta.name,
                    version: meta.version,
                    path: node_path,
                    installed_at: meta.installed_at,
                    description: meta.description,
                    author: meta.author,
                    category: meta.category,
                    inputs: meta.inputs,
                    outputs: meta.outputs,
                    avatar: meta.avatar,
                }))
            }
            Err(_) => {
                // Metadata file missing, return basic info
                Ok(Some(NodeEntry {
                    id: id.to_string(),
                    name: String::new(),
                    version: "unknown".to_string(),
                    path: node_path,
                    installed_at: "unknown".to_string(),
                    description: String::new(),
                    author: None,
                    category: String::new(),
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    avatar: None,
                }))
            }
        }
    })();

    op.emit_result(&result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_list_nodes_empty() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        
        let nodes = list_nodes(home).unwrap();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_node_status_not_installed() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        
        let status = node_status(home, "nonexistent").unwrap();
        assert!(status.is_none());
    }

    #[test]
    fn test_uninstall_nonexistent() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        
        let result = uninstall_node(home, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_install_and_list_and_uninstall() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let id = "test-node";
        
        // Create node directory and metadata manually
        let node_path = node_dir(home, id);
        std::fs::create_dir_all(&node_path).unwrap();
        
        let meta = NodeMetaFile {
            id: id.to_string(),
            name: String::new(),
            version: "1.0.0".to_string(),
            installed_at: "1234567890".to_string(),
            source: NodeSource {
                build: "python".to_string(),
                github: None,
            },
            description: String::new(),
            executable: String::new(),
            author: None,
            category: String::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            avatar: None,
            config_schema: None,
        };
        
        let meta_json = serde_json::to_string_pretty(&meta).unwrap();
        std::fs::write(dm_json_path(home, id), meta_json).unwrap();
        
        // Test list
        let nodes = list_nodes(home).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, id);
        assert_eq!(nodes[0].version, "1.0.0");
        
        // Test status
        let status = node_status(home, id).unwrap().unwrap();
        assert_eq!(status.id, id);
        
        // Test uninstall
        uninstall_node(home, id).unwrap();
        assert!(!node_path.exists());
        
        // Verify empty after uninstall
        let nodes = list_nodes(home).unwrap();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_nodes_dir_path() {
        let home = Path::new("/home/user/.dm");
        assert_eq!(nodes_dir(home), PathBuf::from("/home/user/.dm/nodes"));
    }

    #[test]
    fn test_node_dir_path() {
        let home = Path::new("/home/user/.dm");
        assert_eq!(node_dir(home, "llama-vision"), PathBuf::from("/home/user/.dm/nodes/llama-vision"));
    }

    #[test]
    fn test_dm_json_path() {
        let home = Path::new("/home/user/.dm");
        assert_eq!(dm_json_path(home, "test"), PathBuf::from("/home/user/.dm/nodes/test/dm.json"));
    }
}
