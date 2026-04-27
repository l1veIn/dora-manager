use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::events::{EventSource, OperationEvent};

use super::model::{Service, ServiceFiles, ServiceRuntime, ServiceRuntimeKind, ServiceScope};
use super::paths::{
    builtin_services_dir, configured_service_dirs, resolve_service_dir, service_dir,
};

pub fn create_service(home: &Path, id: &str, description: &str) -> Result<Service> {
    let op = OperationEvent::new(home, EventSource::Core, "service.create").attr("service_id", id);
    op.emit_start();

    let result = (|| {
        let service_path = service_dir(home, id);
        if service_path.exists() {
            bail!(
                "Service '{}' already exists at {}",
                id,
                service_path.display()
            );
        }

        let module_name = id.replace('-', "_");
        let module_dir = service_path.join(&module_name);
        std::fs::create_dir_all(&module_dir).with_context(|| {
            format!(
                "Failed to create module directory: {}",
                module_dir.display()
            )
        })?;

        let pyproject = format!(
            r#"[project]
name = "{id}"
version = "0.1.0"
description = "{description}"
requires-python = ">=3.10"
dependencies = []

[project.scripts]
{id} = "{module_name}.main:main"
"#,
        );
        std::fs::write(service_path.join("pyproject.toml"), pyproject)
            .context("Failed to write pyproject.toml")?;

        let main_py = r#"import json
import sys


def main():
    request = json.load(sys.stdin)
    json.dump({"ok": True, "echo": request}, sys.stdout)


if __name__ == "__main__":
    main()
"#;
        std::fs::write(module_dir.join("main.py"), main_py).context("Failed to write main.py")?;
        std::fs::write(module_dir.join("__init__.py"), "")
            .context("Failed to write __init__.py")?;

        let readme = format!(
            "# {id}\n\n{description}\n\n## Methods\n\n- `echo`: returns the input request.\n",
        );
        std::fs::write(service_path.join("README.md"), readme)
            .context("Failed to write README.md")?;

        let service = Service {
            id: id.to_string(),
            name: id.to_string(),
            version: "0.1.0".to_string(),
            installed_at: String::new(),
            description: description.to_string(),
            repository: None,
            maintainers: Vec::new(),
            license: None,
            display: Default::default(),
            scope: ServiceScope::Global,
            methods: vec![super::model::ServiceMethod {
                name: "echo".to_string(),
                description: "Return the input request.".to_string(),
                input_schema: Some(serde_json::json!({"type": "object"})),
                output_schema: Some(serde_json::json!({"type": "object"})),
            }],
            runtime: ServiceRuntime {
                kind: ServiceRuntimeKind::Command,
                exec: None,
                url: None,
            },
            files: ServiceFiles {
                readme: "README.md".to_string(),
                entry: Some(format!("{module_name}/main.py")),
                config: Some("config.json".to_string()),
                tests: Vec::new(),
                examples: Vec::new(),
            },
            examples: Vec::new(),
            config_schema: None,
            builtin: false,
            path: Default::default(),
        };

        let json =
            serde_json::to_string_pretty(&service).context("Failed to serialize service.json")?;
        std::fs::write(service_path.join("service.json"), json)
            .context("Failed to write service.json")?;

        Ok(service.with_path(service_path))
    })();

    op.emit_result(&result);
    result
}

pub fn list_services(home: &Path) -> Result<Vec<Service>> {
    let op = OperationEvent::new(home, EventSource::Core, "service.list");
    op.emit_start();

    let result = (|| {
        let mut services = Vec::new();
        let mut seen = std::collections::BTreeSet::new();

        for services_path in configured_service_dirs(home) {
            if !services_path.exists() {
                continue;
            }

            for entry in std::fs::read_dir(&services_path)
                .with_context(|| format!("Failed to read directory: {}", services_path.display()))?
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
                if !seen.insert(id.clone()) {
                    continue;
                }

                let service = load_service_from_dir(&path)?.with_path(path.clone());
                services.push(mark_builtin(service, &path));
            }
        }

        services.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(services)
    })();

    op.emit_result(&result);
    result
}

pub fn get_service(home: &Path, id: &str) -> Result<Option<Service>> {
    let op = OperationEvent::new(home, EventSource::Core, "service.status").attr("service_id", id);
    op.emit_start();

    let result = (|| {
        let Some(service_path) = resolve_service_dir(home, id) else {
            return Ok(None);
        };
        let service = load_service_from_dir(&service_path)?.with_path(service_path.clone());
        Ok(Some(mark_builtin(service, &service_path)))
    })();

    op.emit_result(&result);
    result
}

pub fn service_status(home: &Path, id: &str) -> Result<Option<Service>> {
    get_service(home, id)
}

pub fn uninstall_service(home: &Path, id: &str) -> Result<()> {
    let op =
        OperationEvent::new(home, EventSource::Core, "service.uninstall").attr("service_id", id);
    op.emit_start();

    let result = (|| {
        let managed_path = service_dir(home, id);
        if managed_path.exists() {
            std::fs::remove_dir_all(&managed_path).with_context(|| {
                format!(
                    "Failed to remove service directory: {}",
                    managed_path.display()
                )
            })?;
            return Ok(());
        }

        if resolve_service_dir(home, id).is_some() {
            bail!(
                "Service '{}' is builtin and cannot be uninstalled from the managed service directory",
                id
            );
        }

        bail!("Service '{}' is not installed", id);
    })();

    op.emit_result(&result);
    result
}

pub fn get_service_config(home: &Path, id: &str) -> Result<serde_json::Value> {
    let Some(service) = get_service(home, id)? else {
        return Ok(serde_json::json!({}));
    };
    let config_path = service
        .files
        .config
        .as_deref()
        .map(|config| service.path.join(config))
        .unwrap_or_else(|| service.path.join("config.json"));

    if !config_path.exists() {
        return Ok(serde_json::json!({}));
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config for service '{}'", id))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config for service '{}'", id))
}

pub fn save_service_config(home: &Path, id: &str, config: &serde_json::Value) -> Result<()> {
    let service =
        get_service(home, id)?.ok_or_else(|| anyhow::anyhow!("Service '{}' does not exist", id))?;
    if service.builtin {
        bail!("Builtin service '{}' config cannot be modified", id);
    }

    let config_path = service
        .files
        .config
        .as_deref()
        .map(|config| service.path.join(config))
        .unwrap_or_else(|| service.path.join("config.json"));
    let config_json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    std::fs::write(&config_path, config_json)
        .with_context(|| format!("Failed to write config for service '{}'", id))
}

pub fn get_service_readme(home: &Path, id: &str) -> Result<String> {
    let Some(service) = get_service(home, id)? else {
        bail!("Service '{}' does not exist", id);
    };
    let readme_path = service.path.join(&service.files.readme);
    std::fs::read_to_string(&readme_path)
        .with_context(|| format!("Failed to read README for service '{}'", id))
}

pub fn git_like_file_tree(home: &Path, id: &str) -> Result<Vec<String>> {
    let service =
        get_service(home, id)?.ok_or_else(|| anyhow::anyhow!("Service '{}' not found", id))?;
    let mut files = Vec::new();
    collect_service_files(&service.path, &service.path, &mut files)?;
    files.sort();
    Ok(files)
}

pub fn read_service_file(home: &Path, id: &str, file_path: &str) -> Result<String> {
    let service =
        get_service(home, id)?.ok_or_else(|| anyhow::anyhow!("Service '{}' not found", id))?;
    let root = service
        .path
        .canonicalize()
        .with_context(|| format!("Failed to resolve service root for '{}'", id))?;
    let candidate = resolve_safe_service_file(&root, file_path)?;
    std::fs::read_to_string(&candidate)
        .with_context(|| format!("Failed to read service file '{}'", candidate.display()))
}

pub fn read_service_file_bytes(home: &Path, id: &str, file_path: &str) -> Result<Vec<u8>> {
    let service =
        get_service(home, id)?.ok_or_else(|| anyhow::anyhow!("Service '{}' not found", id))?;
    let root = service
        .path
        .canonicalize()
        .with_context(|| format!("Failed to resolve service root for '{}'", id))?;
    let candidate = resolve_safe_service_file(&root, file_path)?;
    std::fs::read(&candidate)
        .with_context(|| format!("Failed to read service file '{}'", candidate.display()))
}

pub(crate) fn load_service_from_dir(service_path: &Path) -> Result<Service> {
    let meta_file = service_path.join("service.json");
    let content = std::fs::read_to_string(&meta_file)
        .with_context(|| format!("Failed to read service manifest: {}", meta_file.display()))?;
    serde_json::from_str::<Service>(&content)
        .with_context(|| format!("Failed to parse service manifest: {}", meta_file.display()))
}

fn mark_builtin(mut service: Service, service_path: &Path) -> Service {
    service.builtin = service.builtin || path_starts_with(service_path, &builtin_services_dir());
    service
}

fn path_starts_with(path: &Path, prefix: &PathBuf) -> bool {
    path.starts_with(prefix)
}

fn collect_service_files(root: &Path, current: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("Failed to read directory: {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if should_skip_service_path(&name) || file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            collect_service_files(root, &path, files)?;
            continue;
        }

        if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .with_context(|| format!("Failed to relativize path '{}'", path.display()))?;
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }

    Ok(())
}

fn resolve_safe_service_file(root: &Path, file_path: &str) -> Result<PathBuf> {
    let requested = Path::new(file_path);
    if file_path.is_empty() || requested.is_absolute() {
        bail!("Invalid service file path '{}'", file_path);
    }

    for component in requested.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("Invalid service file path '{}'", file_path);
            }
        }
    }

    let candidate = root.join(requested);
    let resolved = candidate
        .canonicalize()
        .with_context(|| format!("Failed to resolve service file '{}'", file_path))?;

    if !resolved.starts_with(root) {
        bail!("Invalid service file path '{}'", file_path);
    }

    Ok(resolved)
}

fn should_skip_service_path(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | ".next"
            | ".nuxt"
            | ".svelte-kit"
            | ".venv"
            | "venv"
            | "__pycache__"
            | "node_modules"
            | "dist"
            | "build"
            | "target"
    )
}
