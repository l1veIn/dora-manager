use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::model::{Node, NodeSource};

/// Hints from various sources for dm.json initialization.
#[derive(Default)]
pub struct InitHints {
    /// User-provided description (if creating a new node)
    pub description: Option<String>,
}

/// Initialize dm.json for a node directory.
///
/// Priority chain:
///   existing dm.json (version migrate) > pyproject.toml / Cargo.toml > defaults
pub fn init_dm_json(id: &str, node_path: &Path, hints: InitHints) -> Result<Node> {
    let dm_path = node_path.join("dm.json");

    // 1. If dm.json already exists, migrate and return
    if dm_path.exists() {
        let content = std::fs::read_to_string(&dm_path)
            .with_context(|| format!("Failed to read dm.json at {}", dm_path.display()))?;
        let mut node: Node = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse dm.json at {}", dm_path.display()))?;

        // Ensure id matches directory name
        node.id = id.to_string();
        // Future: apply version migrations here based on node.dm_version

        let json = serde_json::to_string_pretty(&node).context("Failed to serialize dm.json")?;
        std::fs::write(&dm_path, json)
            .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

        return Ok(node.with_path(node_path.to_path_buf()));
    }

    // 2. No dm.json — build from available sources
    let pyproject = parse_pyproject(node_path);
    let cargo = parse_cargo_toml(node_path);

    // Resolve each field with priority: pyproject/cargo > defaults

    let name = pyproject
        .as_ref()
        .map(|p| p.name.clone())
        .or_else(|| cargo.as_ref().map(|c| c.name.clone()))
        .unwrap_or_else(|| id.to_string());

    let description = hints
        .description
        .or_else(|| pyproject.as_ref().and_then(|p| p.description.clone()))
        .or_else(|| cargo.as_ref().and_then(|c| c.description.clone()))
        .unwrap_or_default();

    let version = pyproject
        .as_ref()
        .and_then(|p| p.version.clone())
        .or_else(|| cargo.as_ref().and_then(|c| c.version.clone()))
        .unwrap_or_default();

    let author = pyproject.as_ref().and_then(|p| p.authors.first().cloned());

    let build = infer_build_command(id, &pyproject, &cargo);

    let github = None;

    let category = String::new();
    let inputs = Vec::new();
    let outputs = Vec::new();
    let avatar = None;

    let node = Node {
        id: id.to_string(),
        name,
        version,
        installed_at: super::current_timestamp(),
        source: NodeSource { build, github },
        description,
        executable: String::new(),
        author,
        category,
        inputs,
        outputs,
        avatar,
        config_schema: None,
        dm_version: "1".to_string(),
        path: Default::default(),
    };

    let json = serde_json::to_string_pretty(&node).context("Failed to serialize dm.json")?;
    std::fs::write(&dm_path, json)
        .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

    Ok(node.with_path(node_path.to_path_buf()))
}

// ─── pyproject.toml parsing ───

pub(crate) struct PyProjectInfo {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub build_backend: Option<String>,
}

#[derive(Deserialize)]
struct PyProjectToml {
    project: Option<PyProjectSection>,
    #[serde(rename = "build-system")]
    build_system: Option<BuildSystem>,
}

#[derive(Deserialize)]
struct PyProjectSection {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    authors: Vec<AuthorEntry>,
}

#[derive(Deserialize)]
struct AuthorEntry {
    name: Option<String>,
}

#[derive(Deserialize)]
struct BuildSystem {
    #[serde(rename = "build-backend")]
    build_backend: Option<String>,
}

fn parse_pyproject(node_path: &Path) -> Option<PyProjectInfo> {
    let path = node_path.join("pyproject.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    let toml: PyProjectToml = toml::from_str(&content).ok()?;
    let project = toml.project?;

    Some(PyProjectInfo {
        name: project.name.unwrap_or_default(),
        version: project.version,
        description: project.description,
        authors: project.authors.into_iter().filter_map(|a| a.name).collect(),
        build_backend: toml.build_system.and_then(|bs| bs.build_backend),
    })
}

// ─── Cargo.toml parsing ───

pub(crate) struct CargoInfo {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: Option<String>,
    version: Option<toml::Value>,
    description: Option<toml::Value>,
}

fn parse_cargo_toml(node_path: &Path) -> Option<CargoInfo> {
    let path = node_path.join("Cargo.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    let toml: CargoToml = toml::from_str(&content).ok()?;
    let pkg = toml.package?;

    Some(CargoInfo {
        name: pkg.name.unwrap_or_default(),
        version: pkg.version.and_then(|v| v.as_str().map(String::from)),
        description: pkg.description.and_then(|v| v.as_str().map(String::from)),
    })
}

// ─── Build command inference ───

fn infer_build_command(
    id: &str,
    pyproject: &Option<PyProjectInfo>,
    cargo: &Option<CargoInfo>,
) -> String {
    if let Some(py) = pyproject {
        // maturin backend → install from PyPI (can't build locally without workspace)
        if py.build_backend.as_deref() == Some("maturin") {
            return format!("pip install {}", id);
        }
        // Regular Python project → editable install
        return "pip install -e .".to_string();
    }

    if cargo.is_some() {
        return format!("cargo install {}", id);
    }

    // Fallback: assume PyPI package
    format!("pip install {}", id)
}
