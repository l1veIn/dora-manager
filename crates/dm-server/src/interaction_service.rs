use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InteractionSnapshot {
    #[serde(default)]
    pub displays: Vec<DisplayEntry>,
    #[serde(default)]
    pub inputs: Vec<InputBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayEntry {
    pub node_id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    pub render: String,
    pub tail: bool,
    pub max_lines: usize,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMessage {
    pub seq: i64,
    pub node_id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    pub render: String,
    pub tail: bool,
    pub max_lines: usize,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMessagesResponse {
    pub messages: Vec<DisplayMessage>,
    pub next_seq: i64,
}

#[derive(Debug, Clone)]
pub struct DisplayUpdateResult {
    pub snapshot: InteractionSnapshot,
    pub seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputBinding {
    pub node_id: String,
    pub label: String,
    #[serde(default)]
    pub widgets: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub current_values: BTreeMap<String, serde_json::Value>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    pub seq: i64,
    pub node_id: String,
    pub output_id: String,
    pub value: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct EmitInputEventResult {
    pub snapshot: InteractionSnapshot,
    pub event: InputEvent,
}

#[derive(Debug, Clone)]
pub struct DisplayUpdate {
    pub node_id: String,
    pub label: String,
    pub kind: String,
    pub file: Option<String>,
    pub content: Option<serde_json::Value>,
    pub render: String,
    pub tail: bool,
    pub max_lines: usize,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct InputRegistration {
    pub node_id: String,
    pub label: String,
    pub widgets: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct InputEventWrite {
    pub node_id: String,
    pub output_id: String,
    pub value: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    pub after_seq: Option<i64>,
    pub source_id: Option<String>,
    pub limit: Option<usize>,
}

pub struct InteractionMessageService {
    conn: Mutex<Connection>,
}

impl InteractionMessageService {
    pub fn open(home: &Path, run_id: &str) -> Result<Self> {
        ensure_run_exists(home, run_id)?;
        let db_path = db_path(home, run_id);
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open interaction db at {}", db_path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS display_messages (
                seq         INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id     TEXT NOT NULL,
                label       TEXT NOT NULL,
                kind        TEXT NOT NULL,
                file        TEXT,
                content     TEXT,
                render      TEXT NOT NULL,
                tail        INTEGER NOT NULL,
                max_lines   INTEGER NOT NULL,
                created_at  INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_display_messages_node_id ON display_messages(node_id, seq);
             CREATE TABLE IF NOT EXISTS display_sources (
                node_id      TEXT PRIMARY KEY,
                label        TEXT NOT NULL,
                kind         TEXT NOT NULL,
                file         TEXT,
                content      TEXT,
                render       TEXT NOT NULL,
                tail         INTEGER NOT NULL,
                max_lines    INTEGER NOT NULL,
                updated_at   INTEGER NOT NULL,
                last_seq     INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS input_bindings (
                node_id         TEXT PRIMARY KEY,
                label           TEXT NOT NULL,
                widgets         TEXT NOT NULL,
                current_values  TEXT NOT NULL,
                updated_at      INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS input_events (
                seq         INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id     TEXT NOT NULL,
                output_id   TEXT NOT NULL,
                value       TEXT NOT NULL,
                timestamp   INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_input_events_node_id ON input_events(node_id, seq);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn snapshot(&self) -> Result<InteractionSnapshot> {
        let conn = self.lock()?;
        let mut displays = {
            let mut stmt = conn.prepare(
                "SELECT node_id, label, kind, file, content, render, tail, max_lines, updated_at
                 FROM display_sources
                 ORDER BY label ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(DisplayEntry {
                    node_id: row.get(0)?,
                    label: row.get(1)?,
                    kind: row.get(2)?,
                    file: row.get(3)?,
                    content: parse_optional_json(row.get::<_, Option<String>>(4)?)?,
                    render: row.get(5)?,
                    tail: row.get::<_, i64>(6)? != 0,
                    max_lines: row.get::<_, i64>(7)? as usize,
                    updated_at: row.get(8)?,
                })
            })?;
            collect_rows(rows)?
        };

        let mut inputs = {
            let mut stmt = conn.prepare(
                "SELECT node_id, label, widgets, current_values, updated_at
                 FROM input_bindings
                 ORDER BY label ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                let widgets_raw: String = row.get(2)?;
                let current_values_raw: String = row.get(3)?;
                Ok(InputBinding {
                    node_id: row.get(0)?,
                    label: row.get(1)?,
                    widgets: parse_json_map_sql(&widgets_raw)?,
                    current_values: parse_json_map_sql(&current_values_raw)?,
                    updated_at: row.get(4)?,
                })
            })?;
            collect_rows(rows)?
        };

        // Keep stable ordering for JSON snapshots.
        displays.sort_by(|a, b| a.label.cmp(&b.label));
        inputs.sort_by(|a, b| a.label.cmp(&b.label));

        Ok(InteractionSnapshot { displays, inputs })
    }

    pub fn upsert_display(&self, update: DisplayUpdate) -> Result<DisplayUpdateResult> {
        validate_display(&update)?;
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction()?;
        let content_json = optional_json_string(update.content.as_ref())?;

        tx.execute(
            "INSERT INTO display_messages (node_id, label, kind, file, content, render, tail, max_lines, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                update.node_id,
                update.label,
                update.kind,
                update.file,
                content_json,
                update.render,
                bool_to_i64(update.tail),
                update.max_lines as i64,
                update.timestamp,
            ],
        )?;
        let seq = tx.last_insert_rowid();

        tx.execute(
            "INSERT INTO display_sources (node_id, label, kind, file, content, render, tail, max_lines, updated_at, last_seq)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(node_id) DO UPDATE SET
                label = excluded.label,
                kind = excluded.kind,
                file = excluded.file,
                content = excluded.content,
                render = excluded.render,
                tail = excluded.tail,
                max_lines = excluded.max_lines,
                updated_at = excluded.updated_at,
                last_seq = excluded.last_seq",
            params![
                update.node_id,
                update.label,
                update.kind,
                update.file,
                content_json,
                update.render,
                bool_to_i64(update.tail),
                update.max_lines as i64,
                update.timestamp,
                seq,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        Ok(DisplayUpdateResult {
            snapshot: self.snapshot()?,
            seq,
        })
    }

    pub fn list_display_messages(&self, filter: &MessageFilter) -> Result<DisplayMessagesResponse> {
        let conn = self.lock()?;
        let mut sql = String::from(
            "SELECT seq, node_id, label, kind, file, content, render, tail, max_lines, created_at
             FROM display_messages
             WHERE 1=1",
        );
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(after_seq) = filter.after_seq {
            sql.push_str(" AND seq > ?");
            values.push(Box::new(after_seq));
        }
        if let Some(ref source_id) = filter.source_id {
            sql.push_str(" AND node_id = ?");
            values.push(Box::new(source_id.clone()));
        }
        sql.push_str(" ORDER BY seq ASC LIMIT ?");
        values.push(Box::new(filter.limit.unwrap_or(200) as i64));
        let refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok(DisplayMessage {
                seq: row.get(0)?,
                node_id: row.get(1)?,
                label: row.get(2)?,
                kind: row.get(3)?,
                file: row.get(4)?,
                content: parse_optional_json(row.get::<_, Option<String>>(5)?)?,
                render: row.get(6)?,
                tail: row.get::<_, i64>(7)? != 0,
                max_lines: row.get::<_, i64>(8)? as usize,
                created_at: row.get(9)?,
            })
        })?;
        let messages: Vec<DisplayMessage> = collect_rows(rows)?;
        let next_seq = messages
            .last()
            .map(|message| message.seq)
            .or(filter.after_seq)
            .unwrap_or(0);
        Ok(DisplayMessagesResponse { messages, next_seq })
    }

    pub fn register_input(&self, registration: InputRegistration) -> Result<InteractionSnapshot> {
        let conn = self.lock()?;
        let widgets = serde_json::to_string(&registration.widgets)?;
        conn.execute(
            "INSERT INTO input_bindings (node_id, label, widgets, current_values, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(node_id) DO UPDATE SET
                label = excluded.label,
                widgets = excluded.widgets,
                updated_at = excluded.updated_at",
            params![
                registration.node_id,
                registration.label,
                widgets,
                "{}",
                now_ts(),
            ],
        )?;
        drop(conn);
        self.snapshot()
    }

    pub fn emit_input_event(&self, event: InputEventWrite) -> Result<EmitInputEventResult> {
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction()?;
        let node_id = event.node_id.clone();
        let output_id = event.output_id.clone();
        let value = event.value.clone();
        let timestamp = event.timestamp;
        let current_values = tx
            .query_row(
                "SELECT current_values FROM input_bindings WHERE node_id = ?1",
                params![node_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(current_values_json) = current_values else {
            return Err(anyhow::anyhow!("Unknown input node '{}'", node_id));
        };
        let mut current_values = parse_json_map(&current_values_json)?;
        current_values.insert(output_id.clone(), value.clone());

        tx.execute(
            "UPDATE input_bindings
             SET current_values = ?2, updated_at = ?3
             WHERE node_id = ?1",
            params![
                node_id.clone(),
                serde_json::to_string(&current_values)?,
                timestamp,
            ],
        )?;
        tx.execute(
            "INSERT INTO input_events (node_id, output_id, value, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                node_id.clone(),
                output_id.clone(),
                serde_json::to_string(&value)?,
                timestamp,
            ],
        )?;
        let seq = tx.last_insert_rowid();
        tx.commit()?;
        drop(conn);
        Ok(EmitInputEventResult {
            snapshot: self.snapshot()?,
            event: InputEvent {
                seq,
                node_id,
                output_id,
                value,
                timestamp,
            },
        })
    }

    pub fn claim_input_events(&self, node_id: &str, since: i64) -> Result<Vec<InputEvent>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT seq, node_id, output_id, value, timestamp
             FROM input_events
             WHERE node_id = ?1 AND seq > ?2
             ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map(params![node_id, since], |row| {
            let value_raw: String = row.get(3)?;
            Ok(InputEvent {
                seq: row.get(0)?,
                node_id: row.get(1)?,
                output_id: row.get(2)?,
                value: parse_json_sql(&value_raw)?,
                timestamp: row.get(4)?,
            })
        })?;
        collect_rows(rows)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))
    }
}

pub fn db_path(home: &Path, run_id: &str) -> PathBuf {
    dm_core::runs::run_dir(home, run_id).join("interaction.db")
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

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn ensure_run_exists(home: &Path, run_id: &str) -> Result<()> {
    dm_core::runs::load_run(home, run_id).map(|_| ())
}

fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn optional_json_string(value: Option<&serde_json::Value>) -> Result<Option<String>> {
    value
        .map(serde_json::to_string)
        .transpose()
        .map_err(Into::into)
}

fn parse_optional_json(value: Option<String>) -> rusqlite::Result<Option<serde_json::Value>> {
    value
        .map(|raw| parse_json_sql(&raw))
        .transpose()
}

fn parse_json_map(raw: &str) -> Result<BTreeMap<String, serde_json::Value>> {
    Ok(serde_json::from_str(raw)?)
}

fn parse_json_sql(raw: &str) -> rusqlite::Result<serde_json::Value> {
    serde_json::from_str(raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })
}

fn parse_json_map_sql(raw: &str) -> rusqlite::Result<BTreeMap<String, serde_json::Value>> {
    serde_json::from_str(raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })
}

fn validate_display(update: &DisplayUpdate) -> Result<()> {
    match update.kind.as_str() {
        "file" if update.file.is_none() => Err(anyhow::anyhow!("Display kind=file requires a file path")),
        "inline" if update.content.is_none() => Err(anyhow::anyhow!("Display kind=inline requires content")),
        "file" | "inline" => Ok(()),
        other => Err(anyhow::anyhow!("Unknown display kind '{}'", other)),
    }
}

fn collect_rows<T>(rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>) -> Result<Vec<T>> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
