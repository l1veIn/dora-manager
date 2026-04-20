use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeDm {
    #[serde(default = "default_dm_version")]
    pub version: String,
    #[serde(default)]
    pub bindings: Vec<NodeDmBinding>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeDmBinding {
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default)]
    pub media: Vec<String>,
    #[serde(default)]
    pub lifecycle: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCapabilityDetail {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<NodeCapabilityBinding>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCapabilityBinding {
    #[serde(default)]
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default)]
    pub media: Vec<String>,
    #[serde(default)]
    pub lifecycle: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeCapability {
    Tag(String),
    Detail(NodeCapabilityDetail),
}

impl NodeCapability {
    pub fn name(&self) -> &str {
        match self {
            Self::Tag(name) => name,
            Self::Detail(detail) => &detail.name,
        }
    }

    pub fn bindings(&self) -> &[NodeCapabilityBinding] {
        match self {
            Self::Tag(_) => &[],
            Self::Detail(detail) => &detail.bindings,
        }
    }
}

impl Default for NodeCapability {
    fn default() -> Self {
        Self::Tag(String::new())
    }
}

impl PartialEq<&str> for NodeCapability {
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

impl PartialEq<str> for NodeCapability {
    fn eq(&self, other: &str) -> bool {
        self.name() == other
    }
}

impl PartialEq<String> for NodeCapability {
    fn eq(&self, other: &String) -> bool {
        self.name() == other
    }
}

impl PartialEq<NodeCapability> for &str {
    fn eq(&self, other: &NodeCapability) -> bool {
        *self == other.name()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeInteractionLegacy {
    #[serde(default)]
    pub emit: Vec<String>,
    #[serde(default)]
    pub on: bool,
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
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub multiple: bool,
    /// Port data schema — inline DM Port Schema object or { "$ref": "path" }.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
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
#[derive(Debug, Clone, Serialize)]
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
    /// Deprecated compatibility field for legacy `dm.bindings` payloads.
    /// Reads are normalized into `capabilities`; writes omit this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dm: Option<NodeDm>,
    /// Declared runtime capabilities. Simple coarse tags like `media` remain
    /// strings; richer capability families can carry structured bindings.
    #[serde(default)]
    pub capabilities: Vec<NodeCapability>,
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
    /// When true, this node accepts ports defined at YAML authoring time
    /// that are not pre-declared in `ports`. Schema validation is skipped
    /// for ports not found in `ports`.
    #[serde(default)]
    pub dynamic_ports: bool,
    /// Legacy interaction metadata. Prefer `dm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<NodeInteractionLegacy>,
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

fn default_dm_version() -> String {
    "v0".to_string()
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
            dm: None,
            capabilities: Vec::new(),
            runtime: NodeRuntime::default(),
            ports: Vec::new(),
            files: NodeFiles::default(),
            examples: Vec::new(),
            config_schema: None,
            dynamic_ports: false,
            interaction: None,
            path,
        }
    }

    pub fn capability_bindings(&self) -> Vec<(String, &NodeCapabilityBinding)> {
        self.capabilities
            .iter()
            .filter_map(|capability| match capability {
                NodeCapability::Tag(_) => None,
                NodeCapability::Detail(detail) => Some(
                    detail
                        .bindings
                        .iter()
                        .map(move |binding| (detail.name.clone(), binding)),
                ),
            })
            .flatten()
            .collect()
    }

    pub fn dm_capability_view(&self) -> Option<NodeDm> {
        let bindings = self
            .capabilities
            .iter()
            .filter_map(|capability| match capability {
                NodeCapability::Tag(_) => None,
                NodeCapability::Detail(detail) => Some(detail.bindings.iter().cloned().map(
                    move |binding| NodeDmBinding {
                        family: detail.name.clone(),
                        role: binding.role,
                        port: binding.port,
                        channel: binding.channel,
                        media: binding.media,
                        lifecycle: binding.lifecycle,
                        description: binding.description,
                    },
                )),
            })
            .flatten()
            .collect::<Vec<_>>();

        if bindings.is_empty() {
            None
        } else {
            Some(NodeDm {
                version: default_dm_version(),
                bindings,
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct NodeSerde {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub version: String,
    pub installed_at: String,
    pub source: NodeSource,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub executable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<NodeRepository>,
    #[serde(default)]
    pub maintainers: Vec<NodeMaintainer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default)]
    pub display: NodeDisplay,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dm: Option<NodeDm>,
    #[serde(default)]
    pub capabilities: Vec<NodeCapability>,
    #[serde(default)]
    pub runtime: NodeRuntime,
    #[serde(default)]
    pub ports: Vec<NodePort>,
    #[serde(default)]
    pub files: NodeFiles,
    #[serde(default)]
    pub examples: Vec<NodeExample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub dynamic_ports: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<NodeInteractionLegacy>,
    #[serde(skip_deserializing, default)]
    pub path: PathBuf,
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = NodeSerde::deserialize(deserializer)?;
        let mut capabilities = raw.capabilities;
        if let Some(dm) = raw.dm {
            merge_legacy_dm_into_capabilities(&mut capabilities, dm);
        }

        Ok(Self {
            id: raw.id,
            name: raw.name,
            version: raw.version,
            installed_at: raw.installed_at,
            source: raw.source,
            description: raw.description,
            executable: raw.executable,
            repository: raw.repository,
            maintainers: raw.maintainers,
            license: raw.license,
            display: raw.display,
            dm: None,
            capabilities,
            runtime: raw.runtime,
            ports: raw.ports,
            files: raw.files,
            examples: raw.examples,
            config_schema: raw.config_schema,
            dynamic_ports: raw.dynamic_ports,
            interaction: raw.interaction,
            path: raw.path,
        })
    }
}

fn merge_legacy_dm_into_capabilities(capabilities: &mut Vec<NodeCapability>, dm: NodeDm) {
    let mut legacy_by_family = BTreeMap::<String, Vec<NodeCapabilityBinding>>::new();
    for binding in dm.bindings {
        legacy_by_family
            .entry(binding.family)
            .or_default()
            .push(NodeCapabilityBinding {
                role: binding.role,
                port: binding.port,
                channel: binding.channel,
                media: binding.media,
                lifecycle: binding.lifecycle,
                description: binding.description,
            });
    }

    for (family, bindings) in legacy_by_family {
        let mut merged = false;
        for capability in capabilities.iter_mut() {
            if let NodeCapability::Detail(detail) = capability {
                if detail.name == family {
                    for binding in &bindings {
                        if !detail.bindings.contains(binding) {
                            detail.bindings.push(binding.clone());
                        }
                    }
                    merged = true;
                    break;
                }
            }
        }

        if !merged {
            capabilities.push(NodeCapability::Detail(NodeCapabilityDetail {
                name: family,
                bindings,
            }));
        }
    }
}
