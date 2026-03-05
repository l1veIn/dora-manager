use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};

use super::{Asset, AssetFilter, OutputCommand, PaginatedAssets, PanelSession};

#[derive(Clone)]
pub struct PanelStore {
    run_dir: Arc<PathBuf>,
    conn: Arc<Mutex<Connection>>,
}

impl PanelStore {
    pub fn open(home: &Path, run_id: &str) -> Result<Self> {
        let run_dir = home.join("panel").join(run_id);
        fs::create_dir_all(&run_dir)
            .with_context(|| format!("Failed to create panel run dir {}", run_dir.display()))?;

        let db_path = run_dir.join("index.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open panel db {}", db_path.display()))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS assets (
                seq       INTEGER PRIMARY KEY AUTOINCREMENT,
                input_id  TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                type      TEXT NOT NULL,
                storage   TEXT NOT NULL,
                path      TEXT,
                data      TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_assets_input_seq ON assets(input_id, seq);
            CREATE INDEX IF NOT EXISTS idx_assets_seq       ON assets(seq);

            CREATE TABLE IF NOT EXISTS commands (
                seq       INTEGER PRIMARY KEY AUTOINCREMENT,
                output_id TEXT NOT NULL,
                value     TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_commands_seq ON commands(seq);",
        )?;

        Ok(Self {
            run_dir: Arc::new(run_dir),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn write_asset(&self, input_id: &str, type_hint: &str, data: &[u8]) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let normalized_type = normalize_type(type_hint, data);

        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        if should_inline(&normalized_type, data) {
            let inline_data = String::from_utf8(data.to_vec()).ok();
            conn.execute(
                "INSERT INTO assets (input_id, timestamp, type, storage, path, data)
                 VALUES (?1, ?2, ?3, 'inline', NULL, ?4)",
                params![input_id, now, normalized_type, inline_data],
            )?;
            return Ok(conn.last_insert_rowid());
        }

        conn.execute(
            "INSERT INTO assets (input_id, timestamp, type, storage, path, data)
             VALUES (?1, ?2, ?3, 'file', NULL, NULL)",
            params![input_id, now, normalized_type],
        )?;
        let seq = conn.last_insert_rowid();

        let ext = infer_ext(&normalized_type);
        let rel_path = format!("{}/{:06}.{}", sanitize_fs_component(input_id), seq, ext);
        let full_path = self.run_dir.join(&rel_path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create panel asset dir {}",
                    parent.to_string_lossy()
                )
            })?;
        }

        fs::write(&full_path, data)
            .with_context(|| format!("Failed to write panel asset {}", full_path.display()))?;

        conn.execute(
            "UPDATE assets SET path = ?1 WHERE seq = ?2",
            params![rel_path, seq],
        )?;

        Ok(seq)
    }

    pub fn query_assets(&self, filter: &AssetFilter) -> Result<PaginatedAssets> {
        let limit = filter.limit.unwrap_or(100).clamp(1, 1000);
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        // Build WHERE clauses dynamically
        let mut conditions: Vec<String> = Vec::new();
        let mut params_list: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut param_idx = 1;

        // Determine direction:
        // - before_seq: load older records (backward)
        // - since_seq: load newer records (forward)
        // - neither: load the latest N records (backward, no WHERE on seq)
        let is_backward = filter.before_seq.is_some() || filter.since_seq.is_none();

        if let Some(before) = filter.before_seq {
            conditions.push(format!("seq < ?{}", param_idx));
            params_list.push(Box::new(before));
            param_idx += 1;
        } else if let Some(since) = filter.since_seq {
            conditions.push(format!("seq > ?{}", param_idx));
            params_list.push(Box::new(since));
            param_idx += 1;
        }
        // else: no seq filter → get the latest N records

        if let Some(input_id) = &filter.input_id {
            conditions.push(format!("input_id = ?{}", param_idx));
            params_list.push(Box::new(input_id.clone()));
            param_idx += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // For backward loading: ORDER BY seq DESC to get the N most recent before the cursor
        // Then reverse for chronological display
        let order = if is_backward { "DESC" } else { "ASC" };

        let sql = format!(
            "SELECT seq, input_id, timestamp, type, storage, path, data
             FROM assets {} ORDER BY seq {} LIMIT ?{}",
            where_clause, order, param_idx
        );
        params_list.push(Box::new(limit));

        let total_sql = format!("SELECT COUNT(*) FROM assets {}", where_clause);

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_list.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(Asset {
                seq: row.get(0)?,
                input_id: row.get(1)?,
                timestamp: row.get(2)?,
                type_: row.get(3)?,
                storage: row.get(4)?,
                path: row.get(5)?,
                data: row.get(6)?,
            })
        })?;

        let mut assets = Vec::new();
        for row in rows {
            assets.push(row?);
        }

        // Reverse backward results to chronological order
        if is_backward {
            assets.reverse();
        }

        // Total count uses only the non-limit params
        let total_params_refs: Vec<&dyn rusqlite::ToSql> =
            params_list[..params_list.len() - 1].iter().map(|p| p.as_ref()).collect();
        let total: i64 = conn.query_row(&total_sql, total_params_refs.as_slice(), |row| row.get(0))?;

        Ok(PaginatedAssets { assets, total })
    }

    pub fn write_command(&self, output_id: &str, value: &str) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        conn.execute(
            "INSERT INTO commands (output_id, value, timestamp) VALUES (?1, ?2, ?3)",
            params![output_id, value, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn poll_commands(&self, since_seq: &mut i64) -> Result<Vec<OutputCommand>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT seq, output_id, value, timestamp
             FROM commands
             WHERE seq > ?1
             ORDER BY seq ASC",
        )?;

        let rows = stmt.query_map(params![*since_seq], |row| {
            Ok(OutputCommand {
                seq: row.get(0)?,
                output_id: row.get(1)?,
                value: row.get(2)?,
                timestamp: row.get(3)?,
            })
        })?;

        let mut commands = Vec::new();
        for row in rows {
            let cmd = row?;
            *since_seq = (*since_seq).max(cmd.seq);
            commands.push(cmd);
        }

        Ok(commands)
    }

    pub fn list_sessions(home: &Path) -> Result<Vec<PanelSession>> {
        let base = home.join("panel");
        if !base.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in fs::read_dir(&base).context("Failed to read panel sessions")? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let run_id = entry.file_name().to_string_lossy().to_string();
            let db_path = path.join("index.db");
            if !db_path.exists() {
                continue;
            }

            let conn = Connection::open(&db_path)
                .with_context(|| format!("Failed to open panel db {}", db_path.display()))?;
            let asset_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
                .unwrap_or(0);
            let command_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))
                .unwrap_or(0);

            let disk_size_bytes = dir_size_bytes(&path)?;
            // Use the latest timestamp from DB records instead of filesystem mtime,
            // because opening DB with WAL mode resets directory mtime.
            let last_asset_ts: Option<String> = conn
                .query_row(
                    "SELECT MAX(timestamp) FROM assets",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            let last_cmd_ts: Option<String> = conn
                .query_row(
                    "SELECT MAX(timestamp) FROM commands",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            let last_modified = match (last_asset_ts, last_cmd_ts) {
                (Some(a), Some(c)) => if a > c { a } else { c },
                (Some(a), None) => a,
                (None, Some(c)) => c,
                (None, None) => {
                    // Fallback to filesystem mtime for empty sessions
                    entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Utc> = t.into();
                            dt.to_rfc3339()
                        })
                        .unwrap_or_default()
                }
            };

            sessions.push(PanelSession {
                run_id,
                asset_count,
                command_count,
                disk_size_bytes,
                last_modified,
            });
        }

        sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
        Ok(sessions)
    }

    pub fn clean(home: &Path, keep: usize) -> Result<u32> {
        let sessions = Self::list_sessions(home)?;
        if sessions.len() <= keep {
            let _ = crate::runs::clean_runs(home, keep);
            return Ok(0);
        }

        let mut deleted = 0u32;
        for session in &sessions[keep..] {
            let dir = home.join("panel").join(&session.run_id);
            match fs::remove_dir_all(&dir) {
                Ok(_) => deleted += 1,
                Err(e) => {
                    eprintln!(
                        "Warning: failed to clean panel session {}: {}",
                        session.run_id, e
                    )
                }
            }
        }

        let _ = crate::runs::clean_runs(home, keep);

        Ok(deleted)
    }
}

fn normalize_type(type_hint: &str, data: &[u8]) -> String {
    let hint = type_hint.trim();
    if !hint.is_empty() {
        return hint.to_string();
    }
    if std::str::from_utf8(data).is_ok() {
        "text/plain".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

fn should_inline(content_type: &str, data: &[u8]) -> bool {
    let ct = content_type.to_ascii_lowercase();
    if ct.starts_with("text/") || ct.contains("json") {
        return std::str::from_utf8(data).is_ok();
    }
    false
}

fn sanitize_fs_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "input".to_string()
    } else {
        out
    }
}

fn infer_ext(content_type: &str) -> &'static str {
    match content_type.to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "application/json" | "text/json" => "json",
        "text/plain" => "txt",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/mpeg" => "mp3",
        "video/mp4" => "mp4",
        _ => "bin",
    }
}

fn dir_size_bytes(path: &Path) -> Result<u64> {
    let mut size = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            size += dir_size_bytes(&p)?;
        } else {
            size += meta.len();
        }
    }
    Ok(size)
}
