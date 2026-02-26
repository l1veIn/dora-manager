pub mod config;
pub mod events;
pub mod dora;
pub mod env;
pub mod dataflow;
pub mod install;
pub mod node;
pub mod registry;
pub mod types;
pub mod util;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::Result;

use types::*;

// ─── Public API ───

/// Check environment health
pub async fn doctor(home: &PathBuf) -> Result<DoctorReport> {
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

/// List installed and available versions
pub async fn versions(home: &PathBuf) -> Result<VersionsReport> {
    let cfg = config::load_config(home)?;
    let active = cfg.active_version.as_deref().unwrap_or("");
    let versions_dir = config::versions_dir(home);

    let mut installed: Vec<InstalledVersion> = Vec::new();
    if versions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        installed.push(InstalledVersion {
                            version: name.to_string(),
                            active: name == active,
                        });
                    }
                }
            }
        }
    }
    installed.sort_by(|a, b| a.version.cmp(&b.version));

    let installed_names: Vec<&str> = installed.iter().map(|i| i.version.as_str()).collect();

    let available = match fetch_recent_releases().await {
        Ok(tags) => tags
            .into_iter()
            .map(|tag| {
                let clean = tag.trim_start_matches('v').to_string();
                AvailableVersion {
                    installed: installed_names.contains(&clean.as_str()),
                    tag: clean,
                }
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    Ok(VersionsReport {
        installed,
        available,
    })
}

/// Get runtime status overview
pub async fn status(home: &PathBuf, verbose: bool) -> Result<StatusReport> {
    let cfg = config::load_config(home)?;
    let dm_home = home.display().to_string();

    let (active_version, actual_version) = match &cfg.active_version {
        Some(ver) => {
            let bin = config::versions_dir(home).join(ver).join("dora");
            let actual = dora::get_dora_version(&bin).await.ok();
            (Some(ver.clone()), actual)
        }
        None => (None, None),
    };

    // Runtime check via `dora check`
    let mut runtime_running = false;
    let mut runtime_output = String::new();
    if cfg.active_version.is_some() {
        if let Ok((code, stdout, stderr)) =
            dora::run_dora(home, &["check".to_string()], verbose).await
        {
            runtime_running = code == 0;
            runtime_output = if code == 0 {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            };
        }
    }

    // Dataflow list
    let mut dataflows = Vec::new();
    if cfg.active_version.is_some() {
        if let Ok((code, stdout, _)) = dora::run_dora(home, &["list".to_string()], verbose).await {
            if code == 0 {
                let output = stdout.trim();
                if !output.is_empty() && !output.contains("No running dataflow") {
                    dataflows = output.lines().map(|l| l.to_string()).collect();
                }
            }
        }
    }

    Ok(StatusReport {
        active_version,
        actual_version,
        dm_home,
        runtime_running,
        runtime_output,
        dataflows,
    })
}

/// Start dora coordinator + daemon
pub async fn up(home: &PathBuf, verbose: bool) -> Result<RuntimeResult> {
    let (code, stdout, stderr) = dora::run_dora(home, &["up".to_string()], verbose).await?;
    Ok(RuntimeResult {
        success: code == 0,
        message: if code == 0 {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        },
    })
}

/// Stop dora coordinator + daemon
pub async fn down(home: &PathBuf, verbose: bool) -> Result<RuntimeResult> {
    let (code, stdout, stderr) = dora::run_dora(home, &["destroy".to_string()], verbose).await?;
    Ok(RuntimeResult {
        success: code == 0,
        message: if code == 0 {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        },
    })
}

/// Remove an installed dora version
pub async fn uninstall(home: &PathBuf, version: &str) -> Result<()> {
    let version_dir = config::versions_dir(home).join(version);
    if !version_dir.exists() {
        anyhow::bail!("Version {} is not installed.", version);
    }

    let cfg = config::load_config(home)?;
    if cfg.active_version.as_deref() == Some(version) {
        anyhow::bail!(
            "Cannot uninstall active version {}. Run `dm use <other>` first.",
            version
        );
    }

    std::fs::remove_dir_all(&version_dir)?;
    Ok(())
}

/// Switch active dora version
pub async fn use_version(home: &PathBuf, version: &str) -> Result<String> {
    let version_dir = config::versions_dir(home).join(version);
    let dora_bin = version_dir.join("dora");

    if !dora_bin.exists() {
        anyhow::bail!(
            "Version {} is not installed. Run `dm install {}` first.",
            version,
            version
        );
    }

    let mut cfg = config::load_config(home)?;
    cfg.active_version = Some(version.to_string());
    config::save_config(home, &cfg)?;

    let actual_ver = dora::get_dora_version(&dora_bin).await.unwrap_or_default();

    Ok(actual_ver)
}

/// Setup: check and install prerequisites
pub async fn setup(
    home: &PathBuf,
    verbose: bool,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<InstallProgress>>,
) -> Result<SetupReport> {
    let python = env::check_python().await;
    let uv_check = env::check_uv().await;

    // Try to install uv if not found
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

    // Install dora if not present
    let cfg = config::load_config(home)?;
    let mut dora_installed = cfg.active_version.is_some();
    let mut dora_version = cfg.active_version.clone();

    if !dora_installed {
        match install::install(home, None, verbose, progress_tx).await {
            Ok(result) => {
                dora_installed = true;
                dora_version = Some(result.version);
            }
            Err(_) => {}
        }
    }

    Ok(SetupReport {
        python_installed: python.found,
        uv_installed,
        dora_installed,
        dora_version,
    })
}

/// Pass-through: execute any dora CLI command interactively
pub async fn passthrough(home: &PathBuf, args: &[String], verbose: bool) -> Result<i32> {
    dora::exec_dora(home, args, verbose).await
}

// ─── Internal helpers ───

#[derive(serde::Deserialize)]
struct GithubReleaseTag {
    tag_name: String,
}

async fn fetch_recent_releases() -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/repos/dora-rs/dora/releases?per_page=10")
        .header("User-Agent", "dm/0.1")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("GitHub API returned {}", resp.status());
    }

    let releases: Vec<GithubReleaseTag> = resp.json().await?;
    Ok(releases.into_iter().map(|r| r.tag_name).collect())
}
