pub mod message;

use std::path::{Component, Path, PathBuf};

use anyhow::Result;

pub fn db_path(home: &Path, run_id: &str) -> PathBuf {
    dm_core::runs::run_dir(home, run_id).join("interaction.db")
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn normalize_relative_path(path: &str) -> Result<String> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err(anyhow::anyhow!("Expected path relative to run out dir"));
    }

    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            _ => return Err(anyhow::anyhow!("Invalid relative path")),
        }
    }

    let text = normalized.to_string_lossy().to_string();
    if text.is_empty() {
        return Err(anyhow::anyhow!("Path must not be empty"));
    }
    Ok(text)
}

pub(crate) fn ensure_run_exists(home: &Path, run_id: &str) -> Result<()> {
    dm_core::runs::load_run(home, run_id).map(|_| ())
}

pub(crate) fn parse_json_sql(raw: &str) -> rusqlite::Result<serde_json::Value> {
    serde_json::from_str(raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub(crate) fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
