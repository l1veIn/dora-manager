use std::path::Path;

use anyhow::Result;

use crate::events::{EventSource, OperationEvent};
use crate::{config, dora, types::*};

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

#[derive(serde::Deserialize)]
struct GithubReleaseTag {
    tag_name: String,
}

struct CachedReleases {
    tags: Vec<String>,
    fetched_at: std::time::Instant,
}

const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(600);

async fn fetch_cached_releases() -> Result<Vec<String>> {
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<Option<CachedReleases>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    {
        let guard = cache.lock().unwrap();
        if let Some(ref cached) = *guard {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return Ok(cached.tags.clone());
            }
        }
    }

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
