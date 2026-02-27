use std::path::Path;

use anyhow::Result;

use crate::events::{EventSource, OperationEvent};
use crate::{config, env, install, types::*};

/// Setup: check and install prerequisites
pub async fn setup(
    home: &Path,
    verbose: bool,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<InstallProgress>>,
) -> Result<SetupReport> {
    let op = OperationEvent::new(home, EventSource::Core, "setup");
    op.emit_start();

    let result = async {
        let python = env::check_python().await;
        let uv_check = env::check_uv().await;

        let mut uv_installed = uv_check.found;
        if !uv_installed && python.found {
            let status = tokio::process::Command::new("pip3")
                .args(["install", "uv"])
                .status()
                .await;
            if let Ok(s) = status {
                uv_installed = s.success();
            }
        }

        let cfg = config::load_config(home)?;
        let mut dora_installed = cfg.active_version.is_some();
        let mut dora_version = cfg.active_version.clone();

        if !dora_installed {
            if let Ok(result) = install::install(home, None, verbose, progress_tx).await {
                dora_installed = true;
                dora_version = Some(result.version);
            }
        }

        Ok(SetupReport {
            python_installed: python.found,
            uv_installed,
            dora_installed,
            dora_version,
        })
    }
    .await;

    op.emit_result(&result);
    result
}
