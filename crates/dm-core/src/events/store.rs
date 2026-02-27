use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use super::{export::render_xes, Event, EventFilter};

/// Thread-safe SQLite-backed event store
pub struct EventStore {
    conn: Mutex<Connection>,
}

impl EventStore {
    /// Open (or create) the event database at `<home>/events.db`
    pub fn open(home: &Path) -> Result<Self> {
        std::fs::create_dir_all(home)?;
        let db_path = home.join("events.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open events.db at {}", db_path.display()))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   TEXT    NOT NULL,
                case_id     TEXT    NOT NULL,
                activity    TEXT    NOT NULL,
                source      TEXT    NOT NULL,
                level       TEXT    NOT NULL DEFAULT 'info',
                node_id     TEXT,
                message     TEXT,
                attributes  TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_events_case     ON events(case_id);
            CREATE INDEX IF NOT EXISTS idx_events_source   ON events(source);
            CREATE INDEX IF NOT EXISTS idx_events_time     ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_activity ON events(activity);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert a single event
    pub fn emit(&self, event: &Event) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        conn.execute(
            "INSERT INTO events (timestamp, case_id, activity, source, level, node_id, message, attributes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.timestamp,
                event.case_id,
                event.activity,
                event.source,
                event.level,
                event.node_id,
                event.message,
                event.attributes,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Query events with optional filters
    pub fn query(&self, filter: &EventFilter) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        let mut sql = String::from(
            "SELECT id, timestamp, case_id, activity, source, level, node_id, message, attributes FROM events WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref source) = filter.source {
            sql.push_str(" AND source = ?");
            param_values.push(Box::new(source.clone()));
        }
        if let Some(ref case_id) = filter.case_id {
            sql.push_str(" AND case_id = ?");
            param_values.push(Box::new(case_id.clone()));
        }
        if let Some(ref activity) = filter.activity {
            sql.push_str(" AND activity LIKE ?");
            param_values.push(Box::new(format!("%{}%", activity)));
        }
        if let Some(ref level) = filter.level {
            sql.push_str(" AND level = ?");
            param_values.push(Box::new(level.clone()));
        }
        if let Some(ref node_id) = filter.node_id {
            sql.push_str(" AND node_id = ?");
            param_values.push(Box::new(node_id.clone()));
        }
        if let Some(ref since) = filter.since {
            sql.push_str(" AND timestamp >= ?");
            param_values.push(Box::new(since.clone()));
        }
        if let Some(ref until) = filter.until {
            sql.push_str(" AND timestamp <= ?");
            param_values.push(Box::new(until.clone()));
        }
        if let Some(ref search) = filter.search {
            sql.push_str(" AND (activity LIKE ? OR message LIKE ? OR source LIKE ?)");
            let st = format!("%{}%", search);
            param_values.push(Box::new(st.clone()));
            param_values.push(Box::new(st.clone()));
            param_values.push(Box::new(st));
        }

        sql.push_str(" ORDER BY id DESC");

        let limit = filter.limit.unwrap_or(500);
        sql.push_str(" LIMIT ?");
        param_values.push(Box::new(limit));

        if let Some(offset) = filter.offset {
            sql.push_str(" OFFSET ?");
            param_values.push(Box::new(offset));
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(Event {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                case_id: row.get(2)?,
                activity: row.get(3)?,
                source: row.get(4)?,
                level: row.get(5)?,
                node_id: row.get(6)?,
                message: row.get(7)?,
                attributes: row.get(8)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Count events matching a filter
    pub fn count(&self, filter: &EventFilter) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        let mut sql = String::from("SELECT COUNT(*) FROM events WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref source) = filter.source {
            sql.push_str(" AND source = ?");
            param_values.push(Box::new(source.clone()));
        }
        if let Some(ref case_id) = filter.case_id {
            sql.push_str(" AND case_id = ?");
            param_values.push(Box::new(case_id.clone()));
        }
        if let Some(ref activity) = filter.activity {
            sql.push_str(" AND activity LIKE ?");
            param_values.push(Box::new(format!("%{}%", activity)));
        }
        if let Some(ref level) = filter.level {
            sql.push_str(" AND level = ?");
            param_values.push(Box::new(level.clone()));
        }
        if let Some(ref node_id) = filter.node_id {
            sql.push_str(" AND node_id = ?");
            param_values.push(Box::new(node_id.clone()));
        }
        if let Some(ref since) = filter.since {
            sql.push_str(" AND timestamp >= ?");
            param_values.push(Box::new(since.clone()));
        }
        if let Some(ref until) = filter.until {
            sql.push_str(" AND timestamp <= ?");
            param_values.push(Box::new(until.clone()));
        }
        if let Some(ref search) = filter.search {
            sql.push_str(" AND (activity LIKE ? OR message LIKE ? OR source LIKE ?)");
            let st = format!("%{}%", search);
            param_values.push(Box::new(st.clone()));
            param_values.push(Box::new(st.clone()));
            param_values.push(Box::new(st));
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))?;
        Ok(count)
    }

    /// Export events as XES XML (PM4Py compatible)
    pub fn export_xes(&self, filter: &EventFilter) -> Result<String> {
        let events = self.query(filter)?;
        Ok(render_xes(&events))
    }
}
