use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    /// Relative path to the node executable.
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
    /// Configuration schema for node-level settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
}

/// Source information for an installed node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSource {
    pub build: String,
    pub github: Option<String>,
}

impl NodeMetaFile {
    pub(crate) fn into_entry(self, path: PathBuf) -> NodeEntry {
        NodeEntry {
            id: self.id,
            name: self.name,
            version: self.version,
            path,
            installed_at: self.installed_at,
            description: self.description,
            author: self.author,
            category: self.category,
            inputs: self.inputs,
            outputs: self.outputs,
            avatar: self.avatar,
        }
    }
}

pub(crate) fn fallback_entry(id: String, path: PathBuf) -> NodeEntry {
    NodeEntry {
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
    }
}
