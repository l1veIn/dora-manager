use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use utoipa::ToSchema;

use dm_core::config::{DmConfig, MediaBackend, MediaMtXConfig};

const MEDIAMTX_REPO_API: &str = "https://api.github.com/repos/bluenviron/mediamtx/releases";
const MEDIAMTX_USER_AGENT: &str = "dm-server/0.1";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MediaBackendStatus {
    Disabled,
    Unconfigured,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaStatus {
    pub backend: String,
    pub status: MediaBackendStatus,
    pub enabled: bool,
    pub binary_path: Option<String>,
    pub host: String,
    pub api_port: u16,
    pub rtsp_port: u16,
    pub hls_port: u16,
    pub webrtc_port: u16,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Clone)]
struct ResolvedBinary {
    path: PathBuf,
    version: Option<String>,
    source: &'static str,
}

struct MediaRuntimeState {
    status: MediaStatus,
    child: Option<Child>,
}

pub struct MediaRuntime {
    home: PathBuf,
    config: DmConfig,
    client: Client,
    inner: Mutex<MediaRuntimeState>,
}

impl MediaRuntime {
    pub fn new(home: &Path, config: DmConfig) -> Arc<Self> {
        let status = if !config.media.enabled {
            MediaBackendStatus::Disabled
        } else {
            MediaBackendStatus::Unconfigured
        };
        let enabled = config.media.enabled;
        let mediamtx = config.media.mediamtx.clone();
        Arc::new(Self {
            home: home.to_path_buf(),
            config,
            client: Client::new(),
            inner: Mutex::new(MediaRuntimeState {
                status: MediaStatus {
                    backend: "mediamtx".to_string(),
                    status,
                    enabled,
                    binary_path: None,
                    host: mediamtx.host.clone(),
                    api_port: mediamtx.api_port,
                    rtsp_port: mediamtx.rtsp_port,
                    hls_port: mediamtx.hls_port,
                    webrtc_port: mediamtx.webrtc_port,
                    message: None,
                },
                child: None,
            }),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        if !self.config.media.enabled {
            return Ok(());
        }

        let result = match self.config.media.backend {
            MediaBackend::MediaMtx => self.initialize_mediamtx().await,
        };

        if let Err(err) = &result {
            self.set_error_status(err.to_string()).await;
        }

        result
    }

    pub async fn status(&self) -> MediaStatus {
        self.inner.lock().await.status.clone()
    }

    pub async fn install(&self) -> Result<MediaStatus> {
        let resolved = self.resolve_mediamtx_binary().await?;
        let mut guard = self.inner.lock().await;
        guard.status.binary_path = Some(resolved.path.display().to_string());
        guard.status.message = Some(match resolved.version {
            Some(version) => format!(
                "MediaMTX {} resolved via {}",
                version.trim_start_matches('v'),
                resolved.source
            ),
            None => format!("MediaMTX resolved via {}", resolved.source),
        });
        if !matches!(
            guard.status.status,
            MediaBackendStatus::Ready | MediaBackendStatus::Disabled
        ) {
            guard.status.status = MediaBackendStatus::Unconfigured;
        }
        Ok(guard.status.clone())
    }

    pub fn mediamtx_config(&self) -> &MediaMtXConfig {
        &self.config.media.mediamtx
    }

    pub fn hls_base_url(&self) -> String {
        if let Some(url) = self.config.media.mediamtx.public_hls_url.as_ref() {
            return url.trim_end_matches('/').to_string();
        }

        let host = self
            .config
            .media
            .mediamtx
            .public_host
            .clone()
            .unwrap_or_else(|| self.config.media.mediamtx.host.clone());
        format!("http://{}:{}", host, self.config.media.mediamtx.hls_port)
    }

    pub fn webrtc_base_url(&self) -> String {
        if let Some(url) = self.config.media.mediamtx.public_webrtc_url.as_ref() {
            return url.trim_end_matches('/').to_string();
        }

        let host = self
            .config
            .media
            .mediamtx
            .public_host
            .clone()
            .unwrap_or_else(|| self.config.media.mediamtx.host.clone());
        format!("http://{}:{}", host, self.config.media.mediamtx.webrtc_port)
    }

    async fn initialize_mediamtx(&self) -> Result<()> {
        let resolved = self.resolve_mediamtx_binary().await?;
        let config_path = self.write_mediamtx_config()?;

        let mut command = Command::new(&resolved.path);
        command.arg(config_path);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        let child = command.spawn().with_context(|| {
            format!("failed to start MediaMTX from {}", resolved.path.display())
        })?;

        tokio::time::sleep(Duration::from_millis(250)).await;

        let mut guard = self.inner.lock().await;
        guard.status.status = MediaBackendStatus::Ready;
        guard.status.binary_path = Some(resolved.path.display().to_string());
        guard.status.message = Some(match resolved.version {
            Some(version) => format!(
                "MediaMTX {} ready via {}",
                version.trim_start_matches('v'),
                resolved.source
            ),
            None => format!("MediaMTX ready via {}", resolved.source),
        });
        guard.child = Some(child);
        Ok(())
    }

    async fn resolve_mediamtx_binary(&self) -> Result<ResolvedBinary> {
        if let Ok(path) = std::env::var("DM_MEDIAMTX_PATH") {
            let candidate = PathBuf::from(path);
            if candidate.exists() {
                return Ok(ResolvedBinary {
                    path: candidate,
                    version: None,
                    source: "env",
                });
            }
        }

        if let Some(path) = self.config.media.mediamtx.path.as_ref() {
            let candidate = PathBuf::from(path);
            if candidate.exists() {
                return Ok(ResolvedBinary {
                    path: candidate,
                    version: None,
                    source: "config",
                });
            }
            anyhow::bail!(
                "configured MediaMTX binary not found at {}",
                candidate.display()
            );
        }

        if !self.config.media.mediamtx.auto_download {
            anyhow::bail!("MediaMTX binary path is not configured and auto-download is disabled");
        }

        self.ensure_mediamtx_installed().await
    }

    async fn ensure_mediamtx_installed(&self) -> Result<ResolvedBinary> {
        let release = self
            .fetch_release(self.config.media.mediamtx.version.as_deref())
            .await?;
        let version = release.tag_name.trim_start_matches('v').to_string();
        let binary_name = binary_name();
        let cached_binary = self
            .mediamtx_cache_dir(&version)
            .join(platform_slug())
            .join(binary_name);

        if cached_binary.exists() {
            return Ok(ResolvedBinary {
                path: cached_binary,
                version: Some(version),
                source: "cache",
            });
        }

        let asset = select_mediamtx_asset(&release)
            .with_context(|| format!("no MediaMTX release asset found for {}", platform_slug()))?;
        let install_dir = cached_binary
            .parent()
            .context("invalid MediaMTX cache directory")?
            .to_path_buf();
        self.download_and_extract_asset(asset, &install_dir).await?;
        let extracted_binary =
            find_binary_recursive(&install_dir, binary_name).with_context(|| {
                format!(
                    "could not find {} after extracting {}",
                    binary_name, asset.name
                )
            })?;

        if extracted_binary != cached_binary {
            if let Some(parent) = cached_binary.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(_err) = fs::rename(&extracted_binary, &cached_binary) {
                fs::copy(&extracted_binary, &cached_binary)?;
                fs::remove_file(&extracted_binary)?;
            }
        }

        make_executable(&cached_binary)?;

        Ok(ResolvedBinary {
            path: cached_binary,
            version: Some(version),
            source: "download",
        })
    }

    async fn fetch_release(&self, version: Option<&str>) -> Result<GithubRelease> {
        let url = match version {
            Some(version) => format!("{}/tags/{}", MEDIAMTX_REPO_API, normalize_tag(version)),
            None => format!("{}/latest", MEDIAMTX_REPO_API),
        };

        let mut req = self
            .client
            .get(url)
            .header("User-Agent", MEDIAMTX_USER_AGENT)
            .header("Accept", "application/vnd.github+json");

        if let Ok(token) = env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                req = req.header("Authorization", format!("Bearer {token}"));
            }
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if status.as_u16() == 403 || status.as_u16() == 429 {
                anyhow::bail!(
                    "GitHub API error ({}): {}\n\n  Hint: You may have hit the API rate limit.\n  Set a GitHub personal access token to increase your limit:\n    export GITHUB_TOKEN=ghp_your_token_here",
                    status, body
                );
            }
            anyhow::bail!("GitHub API error ({}): {}", status, body);
        }

        Ok(response.json().await?)
    }

    async fn download_and_extract_asset(
        &self,
        asset: &GithubAsset,
        target_dir: &Path,
    ) -> Result<()> {
        let response = self
            .client
            .get(&asset.browser_download_url)
            .header("User-Agent", MEDIAMTX_USER_AGENT)
            .send()
            .await
            .with_context(|| format!("failed to download {}", asset.browser_download_url))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "failed to download {}: {}",
                asset.browser_download_url,
                response.status()
            );
        }

        let mut bytes = Vec::with_capacity(asset.size as usize);
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            bytes.extend_from_slice(&chunk);
        }

        if target_dir.exists() {
            fs::remove_dir_all(target_dir)?;
        }
        fs::create_dir_all(target_dir)?;

        if asset.name.ends_with(".zip") {
            extract_zip(&bytes, target_dir)?;
        } else if asset.name.ends_with(".tar.gz") {
            extract_tar_gz(&bytes, target_dir)?;
        } else {
            anyhow::bail!("unsupported MediaMTX asset format: {}", asset.name);
        }
        Ok(())
    }

    fn mediamtx_cache_dir(&self, version: &str) -> PathBuf {
        self.home.join("bin").join("mediamtx").join(version)
    }

    fn write_mediamtx_config(&self) -> Result<PathBuf> {
        let mediamtx = &self.config.media.mediamtx;
        let dir = self.home.join("runtime");
        fs::create_dir_all(&dir)?;
        let path = dir.join("mediamtx.generated.yml");
        let config = format!(
            "api: yes\napiAddress: {host}:{api_port}\nrtspAddress: :{rtsp_port}\nhls: yes\nhlsAddress: :{hls_port}\nwebrtc: yes\nwebrtcAddress: :{webrtc_port}\npaths:\n  all:\n    source: publisher\n",
            host = mediamtx.host,
            api_port = mediamtx.api_port,
            rtsp_port = mediamtx.rtsp_port,
            hls_port = mediamtx.hls_port,
            webrtc_port = mediamtx.webrtc_port,
        );
        fs::write(&path, config)?;
        Ok(path)
    }

    async fn set_error_status(&self, message: String) {
        let mut guard = self.inner.lock().await;
        guard.status.status = MediaBackendStatus::Error;
        guard.status.message = Some(message);
    }
}

impl Drop for MediaRuntime {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.inner.try_lock() {
            if let Some(child) = guard.child.as_mut() {
                let _ = child.start_kill();
            }
        }
    }
}

fn normalize_tag(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    }
}

fn platform_slug() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", "x86_64") => "darwin-amd64",
        ("linux", "aarch64") => "linux-arm64",
        ("linux", "x86_64") => "linux-amd64",
        ("windows", "aarch64") => "windows-arm64",
        ("windows", "x86_64") => "windows-amd64",
        _ => "unsupported-platform",
    }
}

fn asset_name_patterns() -> &'static [&'static str] {
    match platform_slug() {
        "darwin-arm64" => &["darwin_arm64", "darwin-arm64"],
        "darwin-amd64" => &["darwin_amd64", "darwin-amd64"],
        "linux-arm64" => &["linux_arm64", "linux-arm64"],
        "linux-amd64" => &["linux_amd64", "linux-amd64"],
        "windows-arm64" => &["windows_arm64", "windows-arm64"],
        "windows-amd64" => &["windows_amd64", "windows-amd64"],
        _ => &[],
    }
}

fn select_mediamtx_asset(release: &GithubRelease) -> Option<&GithubAsset> {
    let patterns = asset_name_patterns();
    release.assets.iter().find(|asset| {
        asset.name.starts_with("mediamtx")
            && patterns.iter().any(|pattern| asset.name.contains(pattern))
            && (asset.name.ends_with(".tar.gz") || asset.name.ends_with(".zip"))
    })
}

fn extract_zip(data: &[u8], target_dir: &Path) -> Result<()> {
    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;
    archive.extract(target_dir)?;
    Ok(())
}

fn extract_tar_gz(data: &[u8], target_dir: &Path) -> Result<()> {
    use std::io::Write;
    use std::process::{Command as StdCommand, Stdio as StdStdio};

    let mut child = StdCommand::new("tar")
        .args(["xzf", "-", "-C"])
        .arg(target_dir)
        .stdin(StdStdio::piped())
        .stdout(StdStdio::null())
        .stderr(StdStdio::piped())
        .spawn()
        .context("failed to launch tar for MediaMTX extraction")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(data)?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!(
            "tar extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn find_binary_recursive(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() && path.file_name().map(|file| file == name).unwrap_or(false) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_binary_recursive(&path, name) {
                return Some(found);
            }
        }
    }
    None
}

fn binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "mediamtx.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "mediamtx"
    }
}

#[cfg_attr(not(unix), allow(unused_variables))]
fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::{
        extract_tar_gz, find_binary_recursive, normalize_tag, platform_slug, select_mediamtx_asset,
    };
    use super::{GithubAsset, GithubRelease};

    #[test]
    fn normalize_tag_adds_v_prefix() {
        assert_eq!(normalize_tag("1.2.3"), "v1.2.3");
        assert_eq!(normalize_tag("v1.2.3"), "v1.2.3");
    }

    #[test]
    fn select_asset_matches_current_platform() {
        let matching = match platform_slug() {
            "darwin-arm64" => "mediamtx_v1.2.3_darwin_arm64.tar.gz",
            "darwin-amd64" => "mediamtx_v1.2.3_darwin_amd64.tar.gz",
            "linux-arm64" => "mediamtx_v1.2.3_linux_arm64.tar.gz",
            "linux-amd64" => "mediamtx_v1.2.3_linux_amd64.tar.gz",
            "windows-arm64" => "mediamtx_v1.2.3_windows_arm64.zip",
            "windows-amd64" => "mediamtx_v1.2.3_windows_amd64.zip",
            _ => "mediamtx_v1.2.3_unknown.tar.gz",
        };
        let release = GithubRelease {
            tag_name: "v1.2.3".to_string(),
            assets: vec![
                GithubAsset {
                    name: "mediamtx_v1.2.3_linux_386.tar.gz".to_string(),
                    browser_download_url: "https://example.invalid/linux386".to_string(),
                    size: 1,
                },
                GithubAsset {
                    name: matching.to_string(),
                    browser_download_url: "https://example.invalid/match".to_string(),
                    size: 1,
                },
            ],
        };

        let asset = select_mediamtx_asset(&release).unwrap();
        assert_eq!(asset.name, matching);
    }

    #[test]
    fn find_binary_recursive_locates_nested_binary() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested").join("bin");
        std::fs::create_dir_all(&nested).unwrap();
        let bin = nested.join("mediamtx");
        std::fs::write(&bin, b"binary").unwrap();

        let found = find_binary_recursive(dir.path(), "mediamtx").unwrap();
        assert_eq!(found, bin);
    }

    #[test]
    fn extract_tar_gz_preserves_flat_archives() {
        let src = tempfile::tempdir().unwrap();
        let tar_dir = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join("mediamtx"), b"binary").unwrap();
        std::fs::write(src.path().join("mediamtx.yml"), b"api: yes\n").unwrap();

        let archive = tar_dir.path().join("mediamtx.tar.gz");
        let status = Command::new("tar")
            .current_dir(src.path())
            .args(["czf"])
            .arg(&archive)
            .args(["mediamtx", "mediamtx.yml"])
            .status()
            .unwrap();
        assert!(status.success());

        let bytes = std::fs::read(&archive).unwrap();
        extract_tar_gz(&bytes, out.path()).unwrap();

        let extracted = out.path().join("mediamtx");
        assert!(extracted.exists(), "expected extracted mediamtx binary");
        assert_eq!(std::fs::read(&extracted).unwrap(), b"binary");
    }
}
