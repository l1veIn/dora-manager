//! Node registry — maps node IDs to their sources.
//!
//! The registry is embedded from `registry.json` at compile time.
//! At runtime, `resolve_node_source()` checks:
//!   1. YAML `source.git` field (highest priority)
//!   2. This registry (local copy or future remote URL)
//!
//! Registry format:
//! ```json
//! {
//!   "nodes": {
//!     "dora-echo": {
//!       "source": { "type": "local", "path": "nodes/dora-echo" }
//!     },
//!     "some-remote-node": {
//!       "source": { "type": "git", "url": "https://github.com/..." }
//!     }
//!   }
//! }
//! ```

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Embedded registry JSON from the repo root.
const REGISTRY_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../registry.json"));

#[derive(Debug, Deserialize)]
struct Registry {
    nodes: std::collections::BTreeMap<String, RegistryEntry>,
}

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    source: RegistrySource,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RegistrySource {
    Local { path: String },
    Git { url: String },
}

/// Look up a node in the registry and return its source.
///
/// For `local` sources, returns the absolute path relative to the repo root
/// (the caller must resolve it against the actual repo/install location).
/// For `git` sources, returns the git URL.
pub fn resolve_node_source(node_id: &str) -> Option<NodeSource> {
    let registry: Registry = serde_json::from_str(REGISTRY_JSON).ok()?;
    let entry = registry.nodes.get(node_id)?;

    Some(match &entry.source {
        RegistrySource::Local { path } => NodeSource::Local(path.clone()),
        RegistrySource::Git { url } => NodeSource::Git(url.clone()),
    })
}

/// List all nodes in the registry.
pub fn list_registry_nodes() -> Vec<String> {
    let registry: Registry = serde_json::from_str(REGISTRY_JSON).unwrap_or(Registry {
        nodes: Default::default(),
    });
    registry.nodes.into_keys().collect()
}

/// Check if a node exists in the registry.
pub fn is_in_registry(node_id: &str) -> bool {
    resolve_node_source(node_id).is_some()
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeSource {
    /// Local path relative to the dm install/repo root.
    Local(String),
    /// Git repository URL.
    Git(String),
}

impl NodeSource {
    /// Resolve a local source to an absolute path given the dm home or repo root.
    pub fn resolve_local(&self, root: &Path) -> Option<PathBuf> {
        match self {
            NodeSource::Local(rel) => {
                let p = root.join(rel);
                if p.exists() { Some(p) } else { None }
            }
            NodeSource::Git(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads() {
        let nodes = list_registry_nodes();
        assert!(!nodes.is_empty(), "registry should contain nodes");
        assert!(nodes.contains(&"dm-display".to_string()));
        assert!(nodes.contains(&"dora-yolo".to_string()));
    }

    #[test]
    fn resolve_known_node() {
        let src = resolve_node_source("dm-display");
        assert!(src.is_some());
        match src.unwrap() {
            NodeSource::Local(path) => assert!(path.contains("dm-display")),
            _ => panic!("expected local source"),
        }
    }

    #[test]
    fn resolve_unknown_returns_none() {
        assert!(resolve_node_source("non-existent-node").is_none());
    }
}
