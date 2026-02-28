use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::events::{EventSource, OperationEvent};
use crate::registry::NodeMeta;

use super::model::NodeMetaFile;
use super::paths::{dm_json_path, node_dir};
use super::NodeEntry;

pub async fn install_node(home: &Path, id: &str) -> Result<NodeEntry> {
    let op = OperationEvent::new(home, EventSource::Core, "node.install").attr("node_id", id);
    op.emit_start();

    let result = async {
        let node_path = node_dir(home, id);
        let dm_path = dm_json_path(home, id);

        if !node_path.exists() || !dm_path.exists() {
            bail!("Node '{}' not found. Download or create it first.", id);
        }

        let dm_content = std::fs::read_to_string(&dm_path)
            .with_context(|| format!("Failed to read dm.json for '{}'", id))?;
        let mut dm_meta: NodeMetaFile = serde_json::from_str(&dm_content)
            .with_context(|| format!("Failed to parse dm.json for '{}'", id))?;

        let build_type = dm_meta.source.build.trim().to_lowercase();
        if build_type.starts_with("pip") || build_type.starts_with("uv") {
            let has_local_pyproject = node_path.join("pyproject.toml").exists();
            let registry_meta = build_meta_from_file(&dm_meta);

            let version = if has_local_pyproject {
                install_local_python_node(&node_path).await?
            } else {
                install_python_node(&registry_meta, &node_path).await?
            };

            dm_meta.version = version;
            dm_meta.executable = if cfg!(windows) {
                format!(".venv/Scripts/{}.exe", id)
            } else {
                format!(".venv/bin/{}", id)
            };
        } else if build_type.starts_with("cargo") {
            let registry_meta = build_meta_from_file(&dm_meta);
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

        dm_meta.installed_at = super::current_timestamp();

        let dm_json =
            serde_json::to_string_pretty(&dm_meta).context("Failed to serialize dm.json")?;
        std::fs::write(&dm_path, dm_json)
            .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

        Ok(dm_meta.into_entry(node_path))
    }
    .await;

    op.emit_result(&result);
    result
}

fn build_meta_from_file(meta: &NodeMetaFile) -> NodeMeta {
    NodeMeta {
        id: meta.id.clone(),
        name: meta.name.clone(),
        build: meta.source.build.clone(),
        description: meta.description.clone(),
        category: meta.category.clone(),
        inputs: meta.inputs.clone(),
        outputs: meta.outputs.clone(),
        system_deps: None,
        tags: Vec::new(),
        github: meta.source.github.clone(),
        source: meta.source.github.clone(),
    }
}

async fn install_local_python_node(node_path: &Path) -> Result<String> {
    let venv_path = node_path.join(".venv");
    let use_uv = Command::new("uv")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
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

    let install_result = if use_uv {
        Command::new("uv")
            .args([
                "pip",
                "install",
                "--python",
                &format!("{}/bin/python", venv_path.display()),
                "-e",
                ".",
            ])
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
        Err(err) => bail!("Failed to run pip install: {}", err),
    }
}

async fn install_python_node(meta: &NodeMeta, node_path: &Path) -> Result<String> {
    let venv_path = node_path.join(".venv");
    let use_uv = Command::new("uv")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
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
        .with_context(|| {
            format!(
                "Failed to create virtual environment at {}",
                venv_path.display()
            )
        })?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Failed to create virtual environment"))?;

    let package_spec = package_spec_from_build(meta);
    let install_result = if use_uv {
        Command::new("uv")
            .args([
                "pip",
                "install",
                "--python",
                &format!("{}/bin/python", venv_path.display()),
                &package_spec,
            ])
            .status()
    } else {
        Command::new(format!("{}/bin/pip", venv_path.display()))
            .args(["install", &package_spec])
            .status()
    };

    match install_result {
        Ok(status) if status.success() => get_python_package_version(&venv_path, &package_spec),
        Ok(_) => bail!("Failed to install package: {}", package_spec),
        Err(err) => bail!("Failed to run pip install: {}", err),
    }
}

fn package_spec_from_build(meta: &NodeMeta) -> String {
    let tokens: Vec<&str> = meta.build.split_whitespace().collect();
    if tokens.starts_with(&["pip", "install"]) || tokens.starts_with(&["uv", "pip", "install"]) {
        if let Some(last) = tokens.last() {
            return (*last).to_string();
        }
    }

    if meta.id.starts_with("dora-") {
        meta.id.clone()
    } else {
        format!("dora-{}", meta.id)
    }
}

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
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(if version.is_empty() {
                "unknown".to_string()
            } else {
                version
            })
        }
        _ => Ok("unknown".to_string()),
    }
}

async fn install_cargo_node(meta: &NodeMeta, node_path: &Path) -> Result<String> {
    let cargo_available = Command::new("cargo")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !cargo_available {
        bail!("Cargo is not installed. Please install Rust first.");
    }

    let package_name = format!("dora-{}", meta.id);
    let status = Command::new("cargo")
        .args([
            "install",
            "--root",
            &node_path.to_string_lossy(),
            &package_name,
        ])
        .status()
        .with_context(|| "Failed to run cargo install")?;

    if !status.success() {
        bail!("Failed to install cargo package: {}", package_name);
    }

    get_crate_version(node_path, &package_name).or_else(|_| Ok("unknown".to_string()))
}

fn get_crate_version(_node_path: &Path, _package: &str) -> Result<String> {
    Ok("unknown".to_string())
}
