use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Source information for a node (build command + optional github URL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSource {
    pub build: String,
    pub github: Option<String>,
}

/// A managed dora node, persisted as `dm.json` in `~/.dm/nodes/{id}/`.
///
/// This is the single source of truth for node metadata:
/// - Serialized to/from `dm.json` on disk
/// - Returned as JSON from the HTTP API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique node identifier (e.g., "dora-keyboard")
    pub id: String,
    /// Human-readable display name
    #[serde(default)]
    pub name: String,
    /// Version of the node
    pub version: String,
    /// Installation timestamp (unix seconds)
    pub installed_at: String,
    /// Build source info
    pub source: NodeSource,
    /// Short description
    #[serde(default)]
    pub description: String,
    /// Relative path to the node executable (empty if not yet installed)
    #[serde(default)]
    pub executable: String,
    /// Author name
    #[serde(default)]
    pub author: Option<String>,
    /// Category tag (e.g., "Visualization", "Peripheral")
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
    /// dm.json schema version for future migration.
    #[serde(default = "default_dm_version")]
    pub dm_version: String,
    /// Runtime-computed absolute path to the node directory.
    /// Not stored in dm.json — populated when loading from disk.
    #[serde(skip_deserializing, default)]
    pub path: PathBuf,
}

fn default_dm_version() -> String {
    "1".to_string()
}

impl Node {
    /// Attach runtime path after loading from disk.
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = path;
        self
    }

    /// Fallback node for directories without a valid dm.json.
    pub fn fallback(id: String, path: PathBuf) -> Self {
        Node {
            id,
            name: String::new(),
            version: "unknown".to_string(),
            installed_at: "unknown".to_string(),
            source: NodeSource {
                build: String::new(),
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
            dm_version: "1".to_string(),
            path,
        }
    }
}
