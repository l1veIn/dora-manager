//! Node Manager - Install, list, and manage local dora nodes
//!
//! Nodes are installed in `~/.dora/nodes/<id>/` with metadata stored in `meta.json`.

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
    /// Version of the installed node
    pub version: String,
    /// Path to the node installation directory
    pub path: PathBuf,
    /// Installation timestamp (ISO 8601 format)
    pub installed_at: String,
}

/// Metadata file stored in each node's directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetaFile {
    pub id: String,
    pub version: String,
    pub installed_at: String,
    pub source: NodeSource,
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

/// Get the metadata file path for a node
pub fn meta_path(home: &Path, id: &str) -> PathBuf {
    node_dir(home, id).join("meta.json")
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

/// Install a node from the registry
///
/// This function:
/// 1. Fetches the registry to find the node
/// 2. Creates the installation directory
/// 3. Installs the node (via pip, cargo, etc. based on build type)
/// 4. Saves metadata
pub async fn install_node(home: &Path, id: &str) -> Result<NodeEntry> {
    let op = OperationEvent::new(home, EventSource::Core, "node.install").attr("node_id", id);
    op.emit_start();

    let result = async {
        // Fetch registry and find the node
        let registry = registry::fetch_registry()
            .await
            .context("Failed to fetch registry")?;

        let meta = registry::find_node(&registry, id)
            .ok_or_else(|| anyhow::anyhow!("Node '{}' not found in registry", id))?;

        let node_path = node_dir(home, id);
        if node_path.exists() {
            bail!("Node '{}' is already installed at {}", id, node_path.display());
        }

        // Create node directory
        std::fs::create_dir_all(&node_path)
            .with_context(|| format!("Failed to create directory: {}", node_path.display()))?;

        // Install based on build type
        let version = install_node_impl(meta, &node_path)
            .await
            .inspect_err(|_| {
                // Clean up on failure
                let _ = std::fs::remove_dir_all(&node_path);
            })?;

        // Save metadata
        let meta_file = NodeMetaFile {
            id: id.to_string(),
            version: version.clone(),
            installed_at: current_timestamp(),
            source: NodeSource {
                build: meta.build.clone(),
                github: None, // Could be extracted from registry in future
            },
        };

        let meta_path = meta_path(home, id);
        let meta_json = serde_json::to_string_pretty(&meta_file)
            .context("Failed to serialize metadata")?;
        std::fs::write(&meta_path, meta_json)
            .with_context(|| format!("Failed to write metadata to {}", meta_path.display()))?;

        Ok(NodeEntry {
            id: id.to_string(),
            version,
            path: node_path,
            installed_at: meta_file.installed_at,
        })
    }
    .await;

    op.emit_result(&result);
    result
}

/// Internal implementation for node installation
async fn install_node_impl(meta: &NodeMeta, node_path: &Path) -> Result<String> {
    let build_cmd = meta.build.trim().to_lowercase();
    if build_cmd.starts_with("pip ") || build_cmd.starts_with("uv ") {
        install_python_node(meta, node_path).await
    } else if build_cmd.starts_with("cargo ") {
        install_cargo_node(meta, node_path).await
    } else {
        bail!(
            "Unsupported build instruction: '{}'. Only 'pip' and 'cargo' commands are supported.",
            meta.build
        )
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
        Command::new(&format!("{}/bin/pip", venv_path.display()))
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
            let meta_file = meta_path(home, &id);
            if let Ok(content) = std::fs::read_to_string(&meta_file) {
                if let Ok(meta) = serde_json::from_str::<NodeMetaFile>(&content) {
                    nodes.push(NodeEntry {
                        id: meta.id,
                        version: meta.version,
                        path,
                        installed_at: meta.installed_at,
                    });
                    continue;
                }
            }

            // Fallback: create entry from directory info
            nodes.push(NodeEntry {
                id,
                version: "unknown".to_string(),
                path,
                installed_at: "unknown".to_string(),
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

/// Get the status of a specific node
pub fn node_status(home: &Path, id: &str) -> Result<Option<NodeEntry>> {
    let op = OperationEvent::new(home, EventSource::Core, "node.status").attr("node_id", id);
    op.emit_start();

    let result = (|| {
        let node_path = node_dir(home, id);

        if !node_path.exists() {
            return Ok(None);
        }

        let meta_file = meta_path(home, id);
        match std::fs::read_to_string(&meta_file) {
            Ok(content) => {
                let meta: NodeMetaFile = serde_json::from_str(&content)
                    .context("Failed to parse node metadata")?;
                Ok(Some(NodeEntry {
                    id: meta.id,
                    version: meta.version,
                    path: node_path,
                    installed_at: meta.installed_at,
                }))
            }
            Err(_) => {
                // Metadata file missing, return basic info
                Ok(Some(NodeEntry {
                    id: id.to_string(),
                    version: "unknown".to_string(),
                    path: node_path,
                    installed_at: "unknown".to_string(),
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
            version: "1.0.0".to_string(),
            installed_at: "1234567890".to_string(),
            source: NodeSource {
                build: "python".to_string(),
                github: None,
            },
        };
        
        let meta_json = serde_json::to_string_pretty(&meta).unwrap();
        std::fs::write(meta_path(home, id), meta_json).unwrap();
        
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
    fn test_meta_path() {
        let home = Path::new("/home/user/.dm");
        assert_eq!(meta_path(home, "test"), PathBuf::from("/home/user/.dm/nodes/test/meta.json"));
    }
}
