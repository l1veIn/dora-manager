use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Source information for a node (build command + optional github URL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSource {
    pub build: String,
    pub github: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeRepository {
    #[serde(default)]
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMaintainer {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeDisplay {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodePortDirection {
    #[default]
    Input,
    Output,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodePort {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub direction: NodePortDirection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub multiple: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeRuntime {
    #[serde(default)]
    pub language: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeFiles {
    #[serde(default = "default_readme_path")]
    pub readme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    #[serde(default)]
    pub tests: Vec<String>,
    #[serde(default)]
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeExample {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub description: String,
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
    /// Canonical repository metadata for the node source tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<NodeRepository>,
    /// Node maintainers shown on the detail page.
    #[serde(default)]
    pub maintainers: Vec<NodeMaintainer>,
    /// SPDX or human-readable license identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Presentation metadata for list/detail pages.
    #[serde(default)]
    pub display: NodeDisplay,
    /// Declared runtime capabilities such as `panel` or `audio`.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Runtime requirements and language metadata.
    #[serde(default)]
    pub runtime: NodeRuntime,
    /// Rich port metadata used by the node detail page and graph tooling.
    #[serde(default)]
    pub ports: Vec<NodePort>,
    /// Well-known files inside the node directory.
    #[serde(default)]
    pub files: NodeFiles,
    /// Example entry points or demo flows associated with the node.
    #[serde(default)]
    pub examples: Vec<NodeExample>,
    /// Configuration schema for node-level settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    /// Runtime-computed absolute path to the node directory.
    /// Not stored in dm.json — populated when loading from disk.
    #[serde(skip_deserializing, default)]
    pub path: PathBuf,
}

fn default_true() -> bool {
    true
}

fn default_readme_path() -> String {
    "README.md".to_string()
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
            repository: None,
            maintainers: Vec::new(),
            license: None,
            display: NodeDisplay::default(),
            capabilities: Vec::new(),
            runtime: NodeRuntime::default(),
            ports: Vec::new(),
            files: NodeFiles::default(),
            examples: Vec::new(),
            config_schema: None,
            path,
        }
    }
}
