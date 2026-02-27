use std::path::Path;

use anyhow::Result;

use crate::events::{EventSource, OperationEvent};
use crate::{config, env, types::*};

/// Check environment health
pub async fn doctor(home: &Path) -> Result<DoctorReport> {
    let op = OperationEvent::new(home, EventSource::Core, "doctor");
    op.emit_start();

    let result = async {
        let python = env::check_python().await;
        let uv = env::check_uv().await;
        let rust = env::check_rust().await;

        let cfg = config::load_config(home)?;
        let versions_dir = config::versions_dir(home);

        let mut installed: Vec<InstalledVersion> = Vec::new();
        if versions_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            installed.push(InstalledVersion {
                                version: name.to_string(),
                                active: cfg.active_version.as_deref() == Some(name),
                            });
                        }
                    }
                }
            }
        }
        installed.sort_by(|a, b| a.version.cmp(&b.version));

        let active_binary_ok = if let Some(ref ver) = cfg.active_version {
            let bin = config::versions_dir(home).join(ver).join("dora");
            bin.exists()
        } else {
            false
        };

        let all_ok = python.found && uv.found && cfg.active_version.is_some() && active_binary_ok;

        Ok(DoctorReport {
            python,
            uv,
            rust,
            installed_versions: installed,
            active_version: cfg.active_version,
            active_binary_ok,
            all_ok,
        })
    }
    .await;

    op.emit_result(&result);
    result
}
