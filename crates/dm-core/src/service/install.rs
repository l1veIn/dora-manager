use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::events::{EventSource, OperationEvent};

use super::local::load_service_from_dir;
use super::model::{Service, ServiceRuntimeKind};
use super::paths::{
    resolve_service_dir, resolve_service_json_path, service_dir, service_json_path,
};

pub async fn install_service(home: &Path, id: &str) -> Result<Service> {
    let op = OperationEvent::new(home, EventSource::Core, "service.install").attr("service_id", id);
    op.emit_start();

    let result = async {
        let service_path = resolve_service_dir(home, id).unwrap_or_else(|| service_dir(home, id));
        let manifest_path =
            resolve_service_json_path(home, id).unwrap_or_else(|| service_json_path(home, id));

        if !service_path.exists() || !manifest_path.exists() {
            bail!("Service '{}' not found. Import or create it first.", id);
        }

        let mut service = load_service_from_dir(&service_path)?;

        match service.runtime.kind {
            ServiceRuntimeKind::Builtin => {
                bail!("Builtin service '{}' does not need installation", id);
            }
            ServiceRuntimeKind::Command => {
                install_local_python_service(&service_path).await?;
                service.runtime.exec = Some(service_executable(id));
            }
            ServiceRuntimeKind::Http
            | ServiceRuntimeKind::Daemon
            | ServiceRuntimeKind::External => {
                bail!(
                    "Service runtime '{:?}' does not support install yet",
                    service.runtime.kind
                );
            }
        }

        service.installed_at = crate::service::current_timestamp();

        let json =
            serde_json::to_string_pretty(&service).context("Failed to serialize service.json")?;
        std::fs::write(&manifest_path, json).with_context(|| {
            format!(
                "Failed to write service.json to {}",
                manifest_path.display()
            )
        })?;

        Ok(service.with_path(service_path))
    }
    .await;

    op.emit_result(&result);
    result
}

async fn install_local_python_service(service_path: &Path) -> Result<()> {
    if !service_path.join("pyproject.toml").exists() {
        bail!(
            "Command service install currently requires pyproject.toml at {}",
            service_path.display()
        );
    }

    let venv_path = service_path.join(".venv");
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
            .current_dir(service_path)
            .status()
    } else {
        Command::new(format!("{}/bin/pip", venv_path.display()))
            .args(["install", "-e", "."])
            .current_dir(service_path)
            .status()
    };

    match install_result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => bail!("Failed to install local service via pip install -e ."),
        Err(err) => bail!("Failed to run pip install: {}", err),
    }
}

fn service_executable(id: &str) -> String {
    if cfg!(windows) {
        format!(".venv/Scripts/{}.exe", id)
    } else {
        format!(".venv/bin/{}", id)
    }
}
