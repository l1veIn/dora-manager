use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::{collect_rows, ensure_run_exists, now_ts, parse_json_sql};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Message {
    pub seq: i64,
    pub from: String,
    pub tag: String,
    pub payload: Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessagesResponse {
    pub messages: Vec<Message>,
    pub next_seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessageSnapshot {
    pub node_id: String,
    pub tag: String,
    pub payload: Value,
    pub seq: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InteractionBinding {
    pub node_id: String,
    pub label: String,
    #[serde(default)]
    pub widgets: BTreeMap<String, Value>,
    #[serde(default)]
    pub current_values: BTreeMap<String, Value>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InteractionStream {
    pub seq: i64,
    pub node_id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    pub render: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    pub after_seq: Option<i64>,
    pub before_seq: Option<i64>,
    pub from: Option<Vec<String>>,
    pub tag: Option<Vec<String>>,
    pub target_to: Option<String>,
    pub limit: Option<usize>,
    pub desc: Option<bool>,
}

pub struct MessageService {
    conn: Mutex<Connection>,
}

impl MessageService {
    pub fn open(home: &Path, run_id: &str) -> Result<Self> {
        ensure_run_exists(home, run_id)?;
        let db_path = super::db_path(home, run_id);
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open interaction db at {}", db_path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS messages (
                seq         INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id     TEXT NOT NULL,
                tag         TEXT NOT NULL,
                payload     TEXT NOT NULL,
                timestamp   INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_messages_node_tag ON messages(node_id, tag, seq);
             CREATE TABLE IF NOT EXISTS message_snapshots (
                node_id     TEXT NOT NULL,
                tag         TEXT NOT NULL,
                payload     TEXT NOT NULL,
                seq         INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL,
                PRIMARY KEY (node_id, tag)
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn push(&self, from: &str, tag: &str, payload: &Value, ts: i64) -> Result<i64> {
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction()?;
        let payload_json = serde_json::to_string(payload)?;

        tx.execute(
            "INSERT INTO messages (node_id, tag, payload, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![from, tag, payload_json, ts],
        )?;
        let seq = tx.last_insert_rowid();

        tx.execute(
            "INSERT INTO message_snapshots (node_id, tag, payload, seq, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(node_id, tag) DO UPDATE SET
                payload = excluded.payload,
                seq = excluded.seq,
                updated_at = excluded.updated_at",
            params![from, tag, payload_json, seq, ts],
        )?;
        tx.commit()?;
        Ok(seq)
    }

    pub fn list(&self, filter: &MessageFilter) -> Result<MessagesResponse> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT seq, node_id, tag, payload, timestamp
             FROM messages
             ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Message {
                seq: row.get(0)?,
                from: row.get(1)?,
                tag: row.get(2)?,
                payload: parse_json_sql(&row.get::<_, String>(3)?)?,
                timestamp: row.get(4)?,
            })
        })?;
        let mut messages: Vec<Message> = collect_rows(rows)?
            .into_iter()
            .filter(|msg| match filter.after_seq {
                Some(after_seq) => msg.seq > after_seq,
                None => true,
            })
            .filter(|msg| match filter.before_seq {
                Some(before_seq) => msg.seq < before_seq,
                None => true,
            })
            .filter(|msg| match filter.from.as_ref() {
                Some(from) => from.iter().any(|item| item == "*" || item == &msg.from),
                None => true,
            })
            .filter(|msg| match filter.tag.as_ref() {
                Some(tags) => tags.iter().any(|item| item == "*" || item == &msg.tag),
                None => true,
            })
            .filter(|msg| match filter.target_to.as_ref() {
                Some(target_to) => msg.payload.get("to").and_then(Value::as_str) == Some(target_to.as_str()),
                None => true,
            })
            .collect();

        if filter.desc.unwrap_or(false) {
            messages.reverse();
        }

        let limit = filter.limit.unwrap_or(200);
        if messages.len() > limit {
            messages.truncate(limit);
        }

        if filter.desc.unwrap_or(false) {
            messages.reverse();
        }

        let next_seq = messages
            .last()
            .map(|m| m.seq)
            .or(filter.after_seq)
            .unwrap_or(0);
        Ok(MessagesResponse { messages, next_seq })
    }

    pub fn snapshots(&self) -> Result<Vec<MessageSnapshot>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT node_id, tag, payload, seq, updated_at
             FROM message_snapshots
             ORDER BY node_id ASC, tag ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MessageSnapshot {
                node_id: row.get(0)?,
                tag: row.get(1)?,
                payload: parse_json_sql(&row.get::<_, String>(2)?)?,
                seq: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn interaction_summary(&self) -> Result<(Vec<InteractionStream>, Vec<InteractionBinding>)> {
        let snapshots = self.snapshots()?;
        let messages = self.list(&MessageFilter {
            tag: Some(vec!["input".to_string()]),
            limit: None,
            ..Default::default()
        })?;

        let mut current_values: BTreeMap<(String, String), Value> = BTreeMap::new();
        for message in messages.messages {
            let Some(target) = message.payload.get("to").and_then(Value::as_str) else {
                continue;
            };
            let Some(output_id) = message.payload.get("output_id").and_then(Value::as_str) else {
                continue;
            };
            let value = message.payload.get("value").cloned().unwrap_or(Value::Null);
            current_values.insert((target.to_string(), output_id.to_string()), value);
        }

        let mut streams = Vec::new();
        let mut inputs = Vec::new();
        for snapshot in snapshots {
            if snapshot.tag == "widgets" {
                let widgets = snapshot
                    .payload
                    .get("widgets")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect::<BTreeMap<_, _>>();
                let label = snapshot
                    .payload
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(&snapshot.node_id)
                    .to_string();
                let current_values = widgets
                    .keys()
                    .filter_map(|output_id| {
                        current_values
                            .get(&(snapshot.node_id.clone(), output_id.clone()))
                            .cloned()
                            .map(|value| (output_id.clone(), value))
                    })
                    .collect();
                inputs.push(InteractionBinding {
                    node_id: snapshot.node_id,
                    label,
                    widgets,
                    current_values,
                    updated_at: snapshot.updated_at,
                });
                continue;
            }

            if snapshot.tag == "input" {
                continue;
            }

            let label = snapshot
                .payload
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(&snapshot.node_id)
                .to_string();
            let file = snapshot
                .payload
                .get("file")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let content = snapshot.payload.get("content").cloned();
            let kind = if file.is_some() { "file" } else { "inline" }.to_string();

            streams.push(InteractionStream {
                seq: snapshot.seq,
                node_id: snapshot.node_id,
                label,
                kind,
                file,
                content,
                render: snapshot.tag,
                created_at: snapshot.updated_at,
            });
        }

        Ok((streams, inputs))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            seq: 0,
            from: String::new(),
            tag: String::new(),
            payload: Value::Null,
            timestamp: now_ts(),
        }
    }
}
