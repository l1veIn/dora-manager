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

use std::path::Path;

use anyhow::{Context, Result};

use crate::events::{EventSource, OperationEvent};
use types::*;

// ─── Public API ───

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

/// List installed and available versions
pub async fn versions(home: &Path) -> Result<VersionsReport> {
    let op = OperationEvent::new(home, EventSource::Core, "versions");
    op.emit_start();

    let result = async {
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

        let available = match fetch_cached_releases().await {
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

        Ok(VersionsReport { installed, available })
    }
    .await;

    op.emit_result(&result);
    result
}

/// Get runtime status overview
pub async fn status(home: &Path, verbose: bool) -> Result<StatusReport> {
    let cfg = config::load_config(home)?;
    let dm_home = home.display().to_string();

    if cfg.active_version.is_none() {
        return Ok(StatusReport {
            active_version: None,
            actual_version: None,
            dm_home,
            runtime_running: false,
            runtime_output: String::new(),
            dataflows: Vec::new(),
        });
    }

    let ver = cfg.active_version.as_ref().unwrap().clone();
    let bin = config::versions_dir(home).join(&ver).join("dora");

    // Run all 3 subprocess calls in parallel
    let check_args = vec!["check".to_string()];
    let list_args = vec!["list".to_string()];
    let (version_result, check_result, list_result) = tokio::join!(
        dora::get_dora_version(&bin),
        dora::run_dora(home, &check_args, verbose),
        dora::run_dora(home, &list_args, verbose),
    );

    let actual_version = version_result.ok();

    let (runtime_running, runtime_output) = match check_result {
        Ok((code, stdout, stderr)) => (
            code == 0,
            if code == 0 { stdout.trim().to_string() } else { stderr.trim().to_string() },
        ),
        _ => (false, String::new()),
    };

    let dataflows = match list_result {
        Ok((code, stdout, _)) if code == 0 => {
            let output = stdout.trim();
            if !output.is_empty() && !output.contains("No running dataflow") {
                output.lines().map(|l| l.to_string()).collect()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };

    Ok(StatusReport {
        active_version: Some(ver),
        actual_version,
        dm_home,
        runtime_running,
        runtime_output,
        dataflows,
    })
}

/// Start dora coordinator + daemon
pub async fn up(home: &Path, verbose: bool) -> Result<RuntimeResult> {
    let op = OperationEvent::new(home, EventSource::Core, "runtime.up");
    op.emit_start();

    let result = async {
        let bin = dora::active_dora_bin(home)?;
        if verbose {
            eprintln!("[dm] exec: {} up", bin.display());
        }

        // Spawn `dora up` without waiting for exit — it stays alive as the coordinator
        let mut child = tokio::process::Command::new(&bin)
            .arg("up")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn dora at {}", bin.display()))?;

        // Poll until the runtime is responsive (up to 5 seconds)
        for i in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // If the process already exited, check if it was an error
            if let Some(exit) = child.try_wait()? {
                if !exit.success() {
                    let stderr = if let Some(mut se) = child.stderr.take() {
                        use tokio::io::AsyncReadExt;
                        let mut buf = String::new();
                        se.read_to_string(&mut buf).await.ok();
                        buf
                    } else {
                        String::new()
                    };
                    return Ok(RuntimeResult {
                        success: false,
                        message: stderr.trim().to_string(),
                    });
                }
            }

            if is_runtime_running(home, verbose).await {
                return Ok(RuntimeResult {
                    success: true,
                    message: "Dora runtime started successfully.".to_string(),
                });
            }
            if verbose {
                eprintln!("[dm] Waiting for runtime to initialize... (attempt {}/10)", i + 1);
            }
        }

        Ok(RuntimeResult {
            success: false,
            message: "Timed out waiting for dora runtime to start.".to_string(),
        })
    }
    .await;

    op.emit_result(&result);
    result
}

/// Stop dora coordinator + daemon
pub async fn down(home: &Path, verbose: bool) -> Result<RuntimeResult> {
    let op = OperationEvent::new(home, EventSource::Core, "runtime.down");
    op.emit_start();

    let result = async {
        let (code, stdout, stderr) = dora::run_dora(home, &["destroy".to_string()], verbose).await?;
        if code != 0 {
            return Ok(RuntimeResult {
                success: false,
                message: stderr.trim().to_string(),
            });
        }

        // Verify the runtime has actually stopped (retry up to 3 times)
        for i in 0..3 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if !is_runtime_running(home, verbose).await {
                return Ok(RuntimeResult {
                    success: true,
                    message: stdout.trim().to_string(),
                });
            }
            if verbose {
                eprintln!("[dm] Waiting for runtime to shut down... (attempt {}/3)", i + 1);
            }
        }

        Ok(RuntimeResult {
            success: false,
            message: "dora destroy returned success but runtime is still running.".to_string(),
        })
    }
    .await;

    op.emit_result(&result);
    result
}

/// Check if dora runtime (coordinator + daemon) is currently running.
pub async fn is_runtime_running(home: &Path, verbose: bool) -> bool {
    if let Ok((code, _, _)) = dora::run_dora(home, &["check".to_string()], verbose).await {
        code == 0
    } else {
        false
    }
}

/// Remove an installed dora version
pub async fn uninstall(home: &Path, version: &str) -> Result<()> {
    let op = OperationEvent::new(home, EventSource::Core, "version.uninstall").attr("version", version);
    op.emit_start();

    let result = async {
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
    .await;

    op.emit_result(&result);
    result
}

/// Switch active dora version
pub async fn use_version(home: &Path, version: &str) -> Result<String> {
    let op = OperationEvent::new(home, EventSource::Core, "version.switch").attr("version", version);
    op.emit_start();

    let result = async {
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
    .await;

    op.emit_result(&result);
    result
}

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

/// Pass-through: execute any dora CLI command interactively
pub async fn passthrough(home: &Path, args: &[String], verbose: bool) -> Result<i32> {
    let op = OperationEvent::new(home, EventSource::Core, "passthrough").attr("args", args);
    op.emit_start();

    let result = dora::exec_dora(home, args, verbose).await;
    op.emit_result(&result);
    result
}

// ─── Internal helpers ───

#[derive(serde::Deserialize)]
struct GithubReleaseTag {
    tag_name: String,
}

/// Cached GitHub releases with TTL
struct CachedReleases {
    tags: Vec<String>,
    fetched_at: std::time::Instant,
}

const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(600); // 10 minutes

/// Get cached releases, fetching from GitHub if expired or empty.
async fn fetch_cached_releases() -> Result<Vec<String>> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<Option<CachedReleases>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    // Check cache
    {
        let guard = cache.lock().unwrap();
        if let Some(ref cached) = *guard {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return Ok(cached.tags.clone());
            }
        }
    }

    // Cache miss or expired — fetch from GitHub
    match fetch_recent_releases().await {
        Ok(tags) => {
            let mut guard = cache.lock().unwrap();
            *guard = Some(CachedReleases {
                tags: tags.clone(),
                fetched_at: std::time::Instant::now(),
            });
            Ok(tags)
        }
        Err(e) => {
            // Fallback to stale cache if available
            let guard = cache.lock().unwrap();
            if let Some(ref cached) = *guard {
                Ok(cached.tags.clone())
            } else {
                Err(e)
            }
        }
    }
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
