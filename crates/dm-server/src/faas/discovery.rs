/// Discover functions from `~/.dm/functions/*/service.json`.
///
/// On startup we scan the configured functions directory and register
/// every valid service.json as a callable function.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

use crate::faas::types::Function;

/// Default location for functions.
pub fn default_functions_dir(home: &std::path::Path) -> PathBuf {
    home.join("functions")
}

/// Scan a directory for function definitions (service.json).
pub fn discover_functions(functions_dir: &Path) -> Result<Vec<Function>> {
    if !functions_dir.exists() {
        info!("functions dir does not exist, creating: {}", functions_dir.display());
        std::fs::create_dir_all(functions_dir)
            .with_context(|| format!("failed to create {}", functions_dir.display()))?;
        return Ok(Vec::new());
    }

    let mut functions = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(functions_dir)
        .with_context(|| format!("failed to read {}", functions_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let dir_path = entry.path();
        let service_path = dir_path.join("service.json");
        if !service_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&service_path)
            .with_context(|| format!("failed to read {}", service_path.display()))?;
        let mut func: Function = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", service_path.display()))?;
        func.path = dir_path;
        func.id = entry.file_name().to_string_lossy().to_string();

        info!("discovered function: {} ({} methods)", func.id, func.methods.len());
        functions.push(func);
    }

    Ok(functions)
}
