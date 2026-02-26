use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Persistent configuration stored at <DM_HOME>/config.toml
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct DmConfig {
    /// Currently active dora version
    pub active_version: Option<String>,
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
