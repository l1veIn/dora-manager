use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::events::{EventSource, OperationEvent};

use super::model::Node;
use super::paths::{dm_json_path, resolve_dm_json_path, resolve_node_dir};

pub async fn install_node(home: &Path, id: &str) -> Result<Node> {
    let op = OperationEvent::new(home, EventSource::Core, "node.install").attr("node_id", id);
    op.emit_start();

    let result = async {
        let node_path =
            resolve_node_dir(home, id).unwrap_or_else(|| super::paths::node_dir(home, id));
        let dm_path = resolve_dm_json_path(home, id).unwrap_or_else(|| dm_json_path(home, id));

        if !node_path.exists() || !dm_path.exists() {
            bail!("Node '{}' not found. Download or create it first.", id);
        }

        let dm_content = std::fs::read_to_string(&dm_path)
            .with_context(|| format!("Failed to read dm.json for '{}'", id))?;
        let mut node: Node = serde_json::from_str(&dm_content)
            .with_context(|| format!("Failed to parse dm.json for '{}'", id))?;

        let build_type = node.source.build.trim().to_lowercase();
        if build_type.starts_with("pip") || build_type.starts_with("uv") {
            let is_local_install = build_type.contains("-e .") || build_type.contains("-e.");

            let version = if is_local_install {
                install_local_python_node(&node_path).await?
            } else {
                install_python_node(&node, &node_path).await?
            };

            node.version = version;
            node.executable = if cfg!(windows) {
                format!(".venv/Scripts/{}.exe", id)
            } else {
                format!(".venv/bin/{}", id)
            };
        } else if build_type.starts_with("cargo") {
            let version = install_cargo_node(&node, &node_path).await?;
            node.version = version;

            let bin_name = if id.starts_with("dora-") {
                id.to_string()
            } else {
                format!("dora-{}", id)
            };
            node.executable = if cfg!(windows) {
                format!("bin/{}.exe", bin_name)
            } else {
                format!("bin/{}", bin_name)
            };
        } else {
            bail!("Unsupported build type: '{}'", node.source.build);
        }

        node.installed_at = super::current_timestamp();

        let dm_json = serde_json::to_string_pretty(&node).context("Failed to serialize dm.json")?;
        std::fs::write(&dm_path, dm_json)
            .with_context(|| format!("Failed to write dm.json to {}", dm_path.display()))?;

        Ok(node.with_path(node_path))
    }
    .await;

    op.emit_result(&result);
    result
}

async fn install_local_python_node(node_path: &Path) -> Result<String> {
    let venv_path = node_path.join(".venv");

    // Remove existing venv to avoid interactive prompt from `uv venv`
    if venv_path.exists() {
        std::fs::remove_dir_all(&venv_path).with_context(|| {
            format!("Failed to remove existing venv at {}", venv_path.display())
        })?;
    }

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

async fn install_python_node(meta: &Node, node_path: &Path) -> Result<String> {
    let venv_path = node_path.join(".venv");

    // Remove existing venv to avoid interactive prompt from `uv venv`
    if venv_path.exists() {
        std::fs::remove_dir_all(&venv_path).with_context(|| {
            format!("Failed to remove existing venv at {}", venv_path.display())
        })?;
    }

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

fn package_spec_from_build(meta: &Node) -> String {
    let tokens: Vec<&str> = meta.source.build.split_whitespace().collect();
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

async fn install_cargo_node(node: &Node, node_path: &Path) -> Result<String> {
    let cargo_available = Command::new("cargo")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !cargo_available {
        bail!("Cargo is not installed. Please install Rust first.");
    }

    let package_name = format!("dora-{}", node.id);
    let build_tokens = node.source.build.split_whitespace().collect::<Vec<_>>();
    let mut command = Command::new("cargo");
    command
        .arg("install")
        .arg("--root")
        .arg(node_path.as_os_str());

    if build_tokens.windows(2).any(|pair| pair == ["--path", "."]) {
        command.arg("--path").arg(".");
        command.current_dir(node_path);
    } else {
        command.arg(&package_name);
    }

    let status = command
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::node::{node_dir, NodeDisplay, NodeFiles, NodeRuntime, NodeSource};
    use crate::test_support::{clear_path, env_lock, set_path};

    use super::{
        get_python_package_version, install_cargo_node, install_local_python_node, install_node,
        install_python_node, package_spec_from_build, Node,
    };

    #[cfg(not(target_os = "windows"))]
    fn write_executable(path: &Path, content: &str) {
        fs::write(path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).unwrap();
        }
    }

    fn sample_node(id: &str, build: &str) -> Node {
        Node {
            id: id.to_string(),
            name: id.to_string(),
            version: String::new(),
            installed_at: "1234567890".to_string(),
            source: NodeSource {
                build: build.to_string(),
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
            path: Default::default(),
        }
    }

    #[test]
    fn package_spec_from_build_uses_explicit_package_or_dora_prefix() {
        assert_eq!(
            package_spec_from_build(&sample_node("demo", "pip install demo-pkg")),
            "demo-pkg"
        );
        assert_eq!(
            package_spec_from_build(&sample_node("demo", "uv pip install demo-pkg")),
            "demo-pkg"
        );
        assert_eq!(
            package_spec_from_build(&sample_node("demo", "python build.py")),
            "dora-demo"
        );
        assert_eq!(
            package_spec_from_build(&sample_node("dora-demo", "python build.py")),
            "dora-demo"
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn get_python_package_version_reads_version_output() {
        let dir = tempdir().unwrap();
        let python = dir.path().join("bin/python");
        fs::create_dir_all(python.parent().unwrap()).unwrap();
        write_executable(&python, "#!/bin/sh\necho 1.2.3\n");

        let version = get_python_package_version(dir.path(), "demo").unwrap();
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn get_python_package_version_returns_unknown_when_command_fails() {
        let dir = tempdir().unwrap();
        let version = get_python_package_version(dir.path(), "demo").unwrap();
        assert_eq!(version, "unknown");
    }

    #[test]
    fn install_cargo_node_errors_when_cargo_is_unavailable() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let _path = clear_path();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(install_cargo_node(
            &sample_node("demo", "cargo install"),
            dir.path(),
        ));

        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cargo is not installed"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn install_local_python_node_uses_uv_and_recreates_existing_venv() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        let node_path = dir.path().join("node");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(node_path.join(".venv/old")).unwrap();
        fs::write(node_path.join(".venv/old/stale.txt"), "stale").unwrap();

        write_executable(
            &bin_dir.join("uv"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo uv 0.1; exit 0; fi\nif [ \"$1\" = \"venv\" ]; then /bin/mkdir -p \"$2/bin\"; printf '#!/bin/sh\\necho 0.0.0\\n' > \"$2/bin/python\"; /bin/chmod +x \"$2/bin/python\"; exit 0; fi\nif [ \"$1\" = \"pip\" ]; then exit 0; fi\nexit 1\n",
        );

        let _path = set_path(bin_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let version = rt.block_on(install_local_python_node(&node_path)).unwrap();

        assert_eq!(version, "0.1.0");
        assert!(!node_path.join(".venv/old/stale.txt").exists());
        assert!(node_path.join(".venv/bin/python").exists());
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn install_python_node_uses_uv_and_reads_installed_version() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        let node_path = dir.path().join("node");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&node_path).unwrap();

        write_executable(
            &bin_dir.join("uv"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo uv 0.1; exit 0; fi\nif [ \"$1\" = \"venv\" ]; then /bin/mkdir -p \"$2/bin\"; printf '#!/bin/sh\\necho 2.3.4\\n' > \"$2/bin/python\"; /bin/chmod +x \"$2/bin/python\"; exit 0; fi\nif [ \"$1\" = \"pip\" ]; then exit 0; fi\nexit 1\n",
        );

        let _path = set_path(bin_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let version = rt
            .block_on(install_python_node(
                &sample_node("demo", "pip install demo-pkg"),
                &node_path,
            ))
            .unwrap();

        assert_eq!(version, "2.3.4");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn install_node_updates_dm_json_for_local_python_installs() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let home = dir.path();
        let bin_dir = home.join("bin");
        let node_path = node_dir(home, "demo");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&node_path).unwrap();

        write_executable(
            &bin_dir.join("uv"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo uv 0.1; exit 0; fi\nif [ \"$1\" = \"venv\" ]; then /bin/mkdir -p \"$2/bin\"; printf '#!/bin/sh\\necho 0.0.0\\n' > \"$2/bin/python\"; /bin/chmod +x \"$2/bin/python\"; exit 0; fi\nif [ \"$1\" = \"pip\" ]; then exit 0; fi\nexit 1\n",
        );

        fs::write(
            node_path.join("dm.json"),
            serde_json::to_string_pretty(&sample_node("demo", "pip install -e .")).unwrap(),
        )
        .unwrap();

        let _path = set_path(bin_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let node = rt.block_on(install_node(home, "demo")).unwrap();

        assert_eq!(node.version, "0.1.0");
        assert_eq!(node.executable, ".venv/bin/demo");

        let persisted: Node =
            serde_json::from_str(&fs::read_to_string(node_path.join("dm.json")).unwrap()).unwrap();
        assert_eq!(persisted.version, "0.1.0");
        assert_eq!(persisted.executable, ".venv/bin/demo");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn install_node_supports_local_cargo_path_builds() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let home = dir.path();
        let bin_dir = home.join("bin");
        let node_path = node_dir(home, "demo");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&node_path).unwrap();

        write_executable(
            &bin_dir.join("cargo"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo cargo 1.0; exit 0; fi\nROOT=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"--root\" ]; then ROOT=\"$2\"; shift 2; continue; fi\n  shift\ndone\n/bin/mkdir -p \"$ROOT/bin\"\nprintf '#!/bin/sh\\necho demo\\n' > \"$ROOT/bin/dora-demo\"\n/bin/chmod +x \"$ROOT/bin/dora-demo\"\nexit 0\n",
        );

        fs::write(
            node_path.join("dm.json"),
            serde_json::to_string_pretty(&sample_node("demo", "cargo install --path .")).unwrap(),
        )
        .unwrap();

        let _path = set_path(bin_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let node = rt.block_on(install_node(home, "demo")).unwrap();

        assert_eq!(node.executable, "bin/dora-demo");
        assert!(node_path.join("bin/dora-demo").exists());

        let persisted: Node =
            serde_json::from_str(&fs::read_to_string(node_path.join("dm.json")).unwrap()).unwrap();
        assert_eq!(persisted.executable, "bin/dora-demo");
    }

    #[tokio::test]
    async fn install_node_errors_for_invalid_dm_json() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let node_path = node_dir(home, "broken");
        fs::create_dir_all(&node_path).unwrap();
        fs::write(node_path.join("dm.json"), "{ invalid").unwrap();

        let err = install_node(home, "broken").await.unwrap_err().to_string();
        assert!(err.contains("Failed to parse dm.json"));
    }
}
