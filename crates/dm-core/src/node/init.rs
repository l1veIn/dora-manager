use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::model::{
    Node, NodeDisplay, NodeFiles, NodeMaintainer, NodeRepository, NodeRuntime, NodeSource,
};

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

    let build = infer_build_command(id, &pyproject, &cargo);
    let repository = infer_repository(&pyproject);
    let maintainers = pyproject
        .as_ref()
        .map(|p| {
            p.authors
                .iter()
                .map(|name| NodeMaintainer {
                    name: name.clone(),
                    email: None,
                    url: None,
                })
                .collect()
        })
        .unwrap_or_default();
    let runtime = infer_runtime(node_path, &pyproject, &cargo);
    let files = infer_files(node_path, id, &pyproject, &cargo);

    let node = Node {
        id: id.to_string(),
        name,
        version,
        installed_at: super::current_timestamp(),
        source: NodeSource {
            build,
            github: repository.as_ref().map(|repo| repo.url.clone()),
        },
        description,
        executable: String::new(),
        repository,
        maintainers,
        license: pyproject.as_ref().and_then(|p| p.license.clone()),
        display: NodeDisplay::default(),
        capabilities: Vec::new(),
        runtime,
        ports: Vec::new(),
        files,
        examples: Vec::new(),
        config_schema: None,
        dynamic_ports: false,
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
    pub license: Option<String>,
    pub requires_python: Option<String>,
    pub repository: Option<String>,
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
    #[serde(rename = "requires-python")]
    requires_python: Option<String>,
    license: Option<LicenseValue>,
    #[serde(default)]
    authors: Vec<AuthorEntry>,
    urls: Option<ProjectUrls>,
}

#[derive(Deserialize)]
struct AuthorEntry {
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LicenseValue {
    String(String),
    Table { text: Option<String> },
}

#[derive(Deserialize)]
struct ProjectUrls {
    #[serde(rename = "Repository")]
    repository: Option<String>,
    #[serde(rename = "repository")]
    repository_lower: Option<String>,
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
        license: project.license.and_then(parse_license),
        requires_python: project.requires_python,
        repository: project
            .urls
            .and_then(|urls| urls.repository.or(urls.repository_lower)),
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

fn parse_license(value: LicenseValue) -> Option<String> {
    match value {
        LicenseValue::String(text) => Some(text),
        LicenseValue::Table { text } => text,
    }
}

fn infer_repository(pyproject: &Option<PyProjectInfo>) -> Option<NodeRepository> {
    pyproject
        .as_ref()
        .and_then(|py| py.repository.as_ref())
        .map(|url| NodeRepository {
            url: url.clone(),
            default_branch: None,
            reference: None,
            subdir: None,
        })
}

fn infer_runtime(
    node_path: &Path,
    pyproject: &Option<PyProjectInfo>,
    cargo: &Option<CargoInfo>,
) -> NodeRuntime {
    let language = if pyproject.is_some() {
        "python"
    } else if cargo.is_some() {
        "rust"
    } else if node_path.join("package.json").exists() {
        "node"
    } else {
        ""
    };

    NodeRuntime {
        language: language.to_string(),
        python: pyproject.as_ref().and_then(|py| py.requires_python.clone()),
        platforms: Vec::new(),
    }
}

fn infer_files(
    node_path: &Path,
    id: &str,
    pyproject: &Option<PyProjectInfo>,
    cargo: &Option<CargoInfo>,
) -> NodeFiles {
    let readme = if node_path.join("README.md").exists() {
        "README.md".to_string()
    } else {
        super::model::NodeFiles::default().readme
    };

    let entry = if pyproject.is_some() {
        let module = id.replace('-', "_");
        let candidates = [
            format!("{module}/main.py"),
            format!("src/{module}/main.py"),
            "main.py".to_string(),
        ];
        candidates
            .into_iter()
            .find(|candidate| node_path.join(candidate).exists())
    } else if cargo.is_some() {
        ["src/main.rs", "main.rs"]
            .into_iter()
            .find(|candidate| node_path.join(candidate).exists())
            .map(str::to_string)
    } else if node_path.join("index.js").exists() {
        Some("index.js".to_string())
    } else {
        None
    };

    let config = ["config.json", "config.toml", "config.yaml", "config.yml"]
        .into_iter()
        .find(|candidate| node_path.join(candidate).exists())
        .map(str::to_string);

    NodeFiles {
        readme,
        entry,
        config,
        tests: collect_named_files(node_path, &["test", "tests"]),
        examples: collect_named_files(node_path, &["example", "examples", "demo"]),
    }
}

fn collect_named_files(node_path: &Path, names: &[&str]) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(node_path) else {
        return Vec::new();
    };

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if !names.iter().any(|candidate| lower.contains(candidate)) {
            continue;
        }

        if path.is_file() || path.is_dir() {
            files.push(name.to_string());
        }
    }
    files.sort();
    files
}
