use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Persistent configuration stored at <DM_HOME>/config.toml
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct DmConfig {
    /// Currently active dora version
    pub active_version: Option<String>,
    #[serde(default)]
    pub media: MediaConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub backend: MediaBackend,
    #[serde(default)]
    pub mediamtx: MediaMtXConfig,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: MediaBackend::MediaMtx,
            mediamtx: MediaMtXConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum MediaBackend {
    #[default]
    MediaMtx,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaMtXConfig {
    pub path: Option<String>,
    pub version: Option<String>,
    #[serde(default = "default_mediamtx_auto_download")]
    pub auto_download: bool,
    #[serde(default = "default_mediamtx_api_port")]
    pub api_port: u16,
    #[serde(default = "default_mediamtx_rtsp_port")]
    pub rtsp_port: u16,
    #[serde(default = "default_mediamtx_hls_port")]
    pub hls_port: u16,
    #[serde(default = "default_mediamtx_webrtc_port")]
    pub webrtc_port: u16,
    #[serde(default = "default_mediamtx_host")]
    pub host: String,
    pub public_host: Option<String>,
    pub public_webrtc_url: Option<String>,
    pub public_hls_url: Option<String>,
}

impl Default for MediaMtXConfig {
    fn default() -> Self {
        Self {
            path: None,
            version: None,
            auto_download: default_mediamtx_auto_download(),
            api_port: default_mediamtx_api_port(),
            rtsp_port: default_mediamtx_rtsp_port(),
            hls_port: default_mediamtx_hls_port(),
            webrtc_port: default_mediamtx_webrtc_port(),
            host: default_mediamtx_host(),
            public_host: None,
            public_webrtc_url: None,
            public_hls_url: None,
        }
    }
}

fn default_mediamtx_api_port() -> u16 {
    9997
}

fn default_mediamtx_auto_download() -> bool {
    true
}

fn default_mediamtx_rtsp_port() -> u16 {
    8554
}

fn default_mediamtx_hls_port() -> u16 {
    8888
}

fn default_mediamtx_webrtc_port() -> u16 {
    8889
}

fn default_mediamtx_host() -> String {
    "127.0.0.1".to_string()
}

/// Resolve the dm home directory.
/// Priority: --home flag > DM_HOME env > ~/.dm
pub fn resolve_home(flag: Option<String>) -> Result<PathBuf> {
    let home = if let Some(h) = flag {
        PathBuf::from(h)
    } else if let Ok(env_home) = std::env::var("DM_HOME") {
        PathBuf::from(env_home)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
            .join(".dm")
    };
    Ok(home)
}

/// Standard subdirectories inside DM_HOME
pub fn versions_dir(home: &Path) -> PathBuf {
    home.join("versions")
}

pub fn active_link(home: &Path) -> PathBuf {
    home.join("active")
}

pub fn config_path(home: &Path) -> PathBuf {
    home.join("config.toml")
}

/// Load config, returning default if file doesn't exist
pub fn load_config(home: &Path) -> Result<DmConfig> {
    let path = config_path(home);
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let cfg: DmConfig = toml::from_str(&content)?;
        Ok(cfg)
    } else {
        Ok(DmConfig::default())
    }
}

/// Save config
pub fn save_config(home: &Path, cfg: &DmConfig) -> Result<()> {
    let path = config_path(home);
    std::fs::create_dir_all(home)?;
    let content = toml::to_string_pretty(cfg)?;
    std::fs::write(&path, content)?;
    Ok(())
}
