use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{collect_rows, ensure_run_exists, optional_json_string, parse_optional_json};

/// A single stream message — used for both message history and source snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StreamMessage {
    pub seq: i64,
    pub node_id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    pub render: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StreamMessagesResponse {
    pub messages: Vec<StreamMessage>,
    pub next_seq: i64,
}

/// Input data for pushing a new stream message.
#[derive(Debug, Clone)]
pub struct StreamPush {
    pub node_id: String,
    pub label: String,
    pub kind: String,
    pub file: Option<String>,
    pub content: Option<serde_json::Value>,
    pub render: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    pub after_seq: Option<i64>,
    pub before_seq: Option<i64>,
    pub source_id: Option<String>,
    pub limit: Option<usize>,
    pub desc: Option<bool>,
}

pub struct StreamService {
    conn: Mutex<Connection>,
}

impl StreamService {
    pub fn open(home: &Path, run_id: &str) -> Result<Self> {
        ensure_run_exists(home, run_id)?;
        let db_path = super::db_path(home, run_id);
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open interaction db at {}", db_path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS stream_messages (
                seq         INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id     TEXT NOT NULL,
                label       TEXT NOT NULL,
                kind        TEXT NOT NULL,
                file        TEXT,
                content     TEXT,
                render      TEXT NOT NULL,
                created_at  INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_stream_messages_node_id ON stream_messages(node_id, seq);
             CREATE TABLE IF NOT EXISTS stream_sources (
                node_id      TEXT PRIMARY KEY,
                label        TEXT NOT NULL,
                kind         TEXT NOT NULL,
                file         TEXT,
                content      TEXT,
                render       TEXT NOT NULL,
                updated_at   INTEGER NOT NULL,
                last_seq     INTEGER NOT NULL
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Push a new stream message. Returns the assigned sequence number.
    pub fn push(&self, msg: StreamPush) -> Result<i64> {
        validate_stream(&msg)?;
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction()?;
        let content_json = optional_json_string(msg.content.as_ref())?;

        tx.execute(
            "INSERT INTO stream_messages (node_id, label, kind, file, content, render, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                msg.node_id,
                msg.label,
                msg.kind,
                msg.file,
                content_json,
                msg.render,
                msg.timestamp,
            ],
        )?;
        let seq = tx.last_insert_rowid();

        tx.execute(
            "INSERT INTO stream_sources (node_id, label, kind, file, content, render, updated_at, last_seq)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(node_id) DO UPDATE SET
                label = excluded.label,
                kind = excluded.kind,
                file = excluded.file,
                content = excluded.content,
                render = excluded.render,
                updated_at = excluded.updated_at,
                last_seq = excluded.last_seq",
            params![
                msg.node_id,
                msg.label,
                msg.kind,
                msg.file,
                content_json,
                msg.render,
                msg.timestamp,
                seq,
            ],
        )?;
        tx.commit()?;
        Ok(seq)
    }

    /// Get the latest state per source node.
    pub fn sources(&self) -> Result<Vec<StreamMessage>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT last_seq, node_id, label, kind, file, content, render, updated_at
             FROM stream_sources
             ORDER BY label ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StreamMessage {
                seq: row.get(0)?,
                node_id: row.get(1)?,
                label: row.get(2)?,
                kind: row.get(3)?,
                file: row.get(4)?,
                content: parse_optional_json(row.get::<_, Option<String>>(5)?)?,
                render: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        collect_rows(rows)
    }

    /// List stream messages with filtering and pagination.
    pub fn list(&self, filter: &MessageFilter) -> Result<StreamMessagesResponse> {
        let conn = self.lock()?;
        let mut sql = String::from(
            "SELECT seq, node_id, label, kind, file, content, render, created_at
             FROM stream_messages
             WHERE 1=1",
        );
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(after_seq) = filter.after_seq {
            sql.push_str(" AND seq > ?");
            values.push(Box::new(after_seq));
        }
        if let Some(before_seq) = filter.before_seq {
            sql.push_str(" AND seq < ?");
            values.push(Box::new(before_seq));
        }
        if let Some(ref source_id) = filter.source_id {
            sql.push_str(" AND node_id = ?");
            values.push(Box::new(source_id.clone()));
        }
        if filter.desc.unwrap_or(false) {
            sql.push_str(" ORDER BY seq DESC LIMIT ?");
        } else {
            sql.push_str(" ORDER BY seq ASC LIMIT ?");
        }
        values.push(Box::new(filter.limit.unwrap_or(200) as i64));
        let refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok(StreamMessage {
                seq: row.get(0)?,
                node_id: row.get(1)?,
                label: row.get(2)?,
                kind: row.get(3)?,
                file: row.get(4)?,
                content: parse_optional_json(row.get::<_, Option<String>>(5)?)?,
                render: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        let mut messages: Vec<StreamMessage> = collect_rows(rows)?;

        if filter.desc.unwrap_or(false) {
            messages.reverse();
        }

        let next_seq = messages
            .last()
            .map(|m| m.seq)
            .or(filter.after_seq)
            .unwrap_or(0);
        Ok(StreamMessagesResponse { messages, next_seq })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))
    }
}

fn validate_stream(msg: &StreamPush) -> Result<()> {
    match msg.kind.as_str() {
        "file" if msg.file.is_none() => {
            Err(anyhow::anyhow!("Stream kind=file requires a file path"))
        }
        "inline" if msg.content.is_none() => {
            Err(anyhow::anyhow!("Stream kind=inline requires content"))
        }
        "file" | "inline" => Ok(()),
        other => Err(anyhow::anyhow!("Unknown stream kind '{}'", other)),
    }
}
