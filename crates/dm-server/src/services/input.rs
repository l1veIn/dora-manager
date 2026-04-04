use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{collect_rows, ensure_run_exists, parse_json_sql};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InputBinding {
    pub node_id: String,
    pub label: String,
    #[serde(default)]
    pub widgets: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub current_values: BTreeMap<String, serde_json::Value>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InputEvent {
    pub seq: i64,
    pub node_id: String,
    pub output_id: String,
    pub value: serde_json::Value,
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

pub struct InputService {
    conn: Mutex<Connection>,
}

impl InputService {
    pub fn open(home: &Path, run_id: &str) -> Result<Self> {
        ensure_run_exists(home, run_id)?;
        let db_path = super::db_path(home, run_id);
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open interaction db at {}", db_path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
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

    /// Get all registered input bindings.
    pub fn bindings(&self) -> Result<Vec<InputBinding>> {
        let conn = self.lock()?;
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
        collect_rows(rows)
    }

    /// Register or update an input binding.
    pub fn register(&self, reg: InputRegistration) -> Result<()> {
        let conn = self.lock()?;
        let widgets = serde_json::to_string(&reg.widgets)?;
        conn.execute(
            "INSERT INTO input_bindings (node_id, label, widgets, current_values, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(node_id) DO UPDATE SET
                label = excluded.label,
                widgets = excluded.widgets,
                updated_at = excluded.updated_at",
            params![reg.node_id, reg.label, widgets, "{}", super::now_ts()],
        )?;
        Ok(())
    }

    /// Emit an input event from the web UI.
    pub fn emit(&self, event: InputEventWrite) -> Result<InputEvent> {
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction()?;

        let current_values = tx
            .query_row(
                "SELECT current_values FROM input_bindings WHERE node_id = ?1",
                params![event.node_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(current_values_json) = current_values else {
            return Err(anyhow::anyhow!("Unknown input node '{}'", event.node_id));
        };
        let mut current_values: BTreeMap<String, serde_json::Value> =
            serde_json::from_str(&current_values_json)?;
        current_values.insert(event.output_id.clone(), event.value.clone());

        tx.execute(
            "UPDATE input_bindings SET current_values = ?2, updated_at = ?3 WHERE node_id = ?1",
            params![
                event.node_id,
                serde_json::to_string(&current_values)?,
                event.timestamp,
            ],
        )?;
        tx.execute(
            "INSERT INTO input_events (node_id, output_id, value, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                event.node_id,
                event.output_id,
                serde_json::to_string(&event.value)?,
                event.timestamp,
            ],
        )?;
        let seq = tx.last_insert_rowid();
        tx.commit()?;

        Ok(InputEvent {
            seq,
            node_id: event.node_id,
            output_id: event.output_id,
            value: event.value,
            timestamp: event.timestamp,
        })
    }

    /// Claim input events for a specific node since a given sequence.
    pub fn claim(&self, node_id: &str, since: i64) -> Result<Vec<InputEvent>> {
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

fn parse_json_map_sql(raw: &str) -> rusqlite::Result<BTreeMap<String, serde_json::Value>> {
    serde_json::from_str(raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
