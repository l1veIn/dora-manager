//! Unified Event Store — XES-compatible observability infrastructure
//!
//! All observability data (system logs, dataflow execution logs, HTTP request logs,
//! frontend analytics, CI metrics) is stored as events in a single SQLite table.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Event Model ───

/// Event source classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventSource {
    Core,
    Dataflow,
    Server,
    Frontend,
    Ci,
}

impl std::fmt::Display for EventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Dataflow => write!(f, "dataflow"),
            Self::Server => write!(f, "server"),
            Self::Frontend => write!(f, "frontend"),
            Self::Ci => write!(f, "ci"),
        }
    }
}

impl std::str::FromStr for EventSource {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "core" => Ok(Self::Core),
            "dataflow" => Ok(Self::Dataflow),
            "server" => Ok(Self::Server),
            "frontend" => Ok(Self::Frontend),
            "ci" => Ok(Self::Ci),
            _ => anyhow::bail!("Unknown event source: {}", s),
        }
    }
}

/// Event severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for EventLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "trace"),
            Self::Debug => write!(f, "debug"),
            Self::Info => write!(f, "info"),
            Self::Warn => write!(f, "warn"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for EventLevel {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => anyhow::bail!("Unknown event level: {}", s),
        }
    }
}

/// A single observability event (XES-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Auto-increment ID (set by DB, 0 before insert)
    pub id: i64,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// XES case identifier (session_id / dataflow_id / request_id / user_id / commit_hash)
    pub case_id: String,
    /// XES activity name (e.g. "node.install", "ui.click", "clippy.warn")
    pub activity: String,
    /// Source classification
    pub source: String,
    /// Severity level
    pub level: String,
    /// Optional dora node id
    pub node_id: Option<String>,
    /// Human-readable message
    pub message: Option<String>,
    /// Arbitrary JSON attributes
    pub attributes: Option<String>,
}

/// Builder for creating events ergonomically
pub struct EventBuilder {
    case_id: String,
    activity: String,
    source: EventSource,
    level: EventLevel,
    node_id: Option<String>,
    message: Option<String>,
    attributes: Option<serde_json::Value>,
}

impl EventBuilder {
    pub fn new(source: EventSource, activity: impl Into<String>) -> Self {
        Self {
            case_id: String::new(),
            activity: activity.into(),
            source,
            level: EventLevel::Info,
            node_id: None,
            message: None,
            attributes: None,
        }
    }

    pub fn case_id(mut self, id: impl Into<String>) -> Self {
        self.case_id = id.into();
        self
    }

    pub fn level(mut self, level: EventLevel) -> Self {
        self.level = level;
        self
    }

    pub fn node_id(mut self, id: impl Into<String>) -> Self {
        self.node_id = Some(id.into());
        self
    }

    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    pub fn attr(mut self, key: &str, value: impl Serialize) -> Self {
        let map = self.attributes.get_or_insert_with(|| serde_json::json!({}));
        if let Some(obj) = map.as_object_mut() {
            obj.insert(key.to_string(), serde_json::to_value(value).unwrap_or_default());
        }
        self
    }

    pub fn build(self) -> Event {
        Event {
            id: 0,
            timestamp: Utc::now().to_rfc3339(),
            case_id: self.case_id,
            activity: self.activity,
            source: self.source.to_string(),
            level: self.level.to_string(),
            node_id: self.node_id,
            message: self.message,
            attributes: self.attributes.map(|v| v.to_string()),
        }
    }
}

// ─── Query Filter ───

/// Filter for querying events
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    pub source: Option<String>,
    pub case_id: Option<String>,
    pub activity: Option<String>,
    pub level: Option<String>,
    pub node_id: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<i64>,
}

// ─── EventStore ───

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

        // Enable WAL for concurrent read performance
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

        let mut sql = String::from("SELECT id, timestamp, case_id, activity, source, level, node_id, message, attributes FROM events WHERE 1=1");
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

        sql.push_str(" ORDER BY id DESC");

        let limit = filter.limit.unwrap_or(500);
        sql.push_str(" LIMIT ?");
        param_values.push(Box::new(limit));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

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

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))?;
        Ok(count)
    }

    /// Export events as XES XML (PM4Py compatible)
    pub fn export_xes(&self, filter: &EventFilter) -> Result<String> {
        let events = self.query(filter)?;

        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<log xes.version="1.0" xes.features="nested-attributes" xmlns="http://www.xes-standard.org/">
  <extension name="Concept" prefix="concept" uri="http://www.xes-standard.org/concept.xesext"/>
  <extension name="Time" prefix="time" uri="http://www.xes-standard.org/time.xesext"/>
  <extension name="Lifecycle" prefix="lifecycle" uri="http://www.xes-standard.org/lifecycle.xesext"/>
"#);

        // Group events by case_id into traces
        let mut cases: std::collections::BTreeMap<String, Vec<&Event>> = std::collections::BTreeMap::new();
        for event in &events {
            cases.entry(event.case_id.clone()).or_default().push(event);
        }

        for (case_id, trace_events) in &cases {
            xml.push_str(&format!(
                "  <trace>\n    <string key=\"concept:name\" value=\"{}\"/>\n",
                escape_xml(case_id)
            ));

            for event in trace_events {
                xml.push_str("    <event>\n");
                xml.push_str(&format!(
                    "      <string key=\"concept:name\" value=\"{}\"/>\n",
                    escape_xml(&event.activity)
                ));
                xml.push_str(&format!(
                    "      <date key=\"time:timestamp\" value=\"{}\"/>\n",
                    escape_xml(&event.timestamp)
                ));
                xml.push_str(&format!(
                    "      <string key=\"source\" value=\"{}\"/>\n",
                    escape_xml(&event.source)
                ));
                xml.push_str(&format!(
                    "      <string key=\"level\" value=\"{}\"/>\n",
                    escape_xml(&event.level)
                ));
                if let Some(ref node_id) = event.node_id {
                    xml.push_str(&format!(
                        "      <string key=\"node_id\" value=\"{}\"/>\n",
                        escape_xml(node_id)
                    ));
                }
                if let Some(ref message) = event.message {
                    xml.push_str(&format!(
                        "      <string key=\"message\" value=\"{}\"/>\n",
                        escape_xml(message)
                    ));
                }
                xml.push_str("    </event>\n");
            }

            xml.push_str("  </trace>\n");
        }

        xml.push_str("</log>\n");
        Ok(xml)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Try to emit an event, silently ignoring failures.
pub fn try_emit(home: &Path, event: Event) {
    if let Ok(store) = EventStore::open(home) {
        let _ = store.emit(&event);
    }
}

/// Helper for emitting start/end events around a single operation.
pub struct OperationEvent {
    home: PathBuf,
    source: EventSource,
    activity: String,
    case_id: String,
    attrs: Vec<(String, serde_json::Value)>,
}

impl OperationEvent {
    pub fn new(home: &Path, source: EventSource, activity: impl Into<String>) -> Self {
        Self {
            home: home.to_path_buf(),
            source,
            activity: activity.into(),
            case_id: format!("session_{}", Uuid::new_v4()),
            attrs: Vec::new(),
        }
    }

    pub fn attr(mut self, key: &str, value: impl Serialize) -> Self {
        self.attrs.push((
            key.to_string(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        ));
        self
    }

    fn builder(&self) -> EventBuilder {
        let mut builder =
            EventBuilder::new(self.source.clone(), self.activity.clone()).case_id(self.case_id.clone());
        for (key, value) in &self.attrs {
            builder = builder.attr(key, value.clone());
        }
        builder
    }

    pub fn emit_start(&self) {
        try_emit(&self.home, self.builder().message("START").build());
    }

    pub fn emit_result<T>(&self, result: &Result<T>) {
        let builder = match result {
            Ok(_) => self.builder().level(EventLevel::Info).message("OK"),
            Err(err) => self
                .builder()
                .level(EventLevel::Error)
                .message(err.to_string()),
        };
        try_emit(&self.home, builder.build());
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_store() -> (tempfile::TempDir, EventStore) {
        let dir = tempdir().unwrap();
        let store = EventStore::open(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn emit_and_query() {
        let (_dir, store) = test_store();

        let event = EventBuilder::new(EventSource::Core, "node.install")
            .case_id("session_001")
            .message("Installing opencv-video-capture")
            .attr("version", "0.4.1")
            .build();

        let id = store.emit(&event).unwrap();
        assert!(id > 0);

        let results = store.query(&EventFilter::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].activity, "node.install");
        assert_eq!(results[0].source, "core");
    }

    #[test]
    fn filter_by_source() {
        let (_dir, store) = test_store();

        store.emit(&EventBuilder::new(EventSource::Core, "version.switch").case_id("s1").build()).unwrap();
        store.emit(&EventBuilder::new(EventSource::Dataflow, "node.spawn").case_id("df1").build()).unwrap();
        store.emit(&EventBuilder::new(EventSource::Frontend, "ui.click").case_id("u1").build()).unwrap();

        let core_events = store.query(&EventFilter { source: Some("core".into()), ..Default::default() }).unwrap();
        assert_eq!(core_events.len(), 1);

        let all_events = store.query(&EventFilter::default()).unwrap();
        assert_eq!(all_events.len(), 3);
    }

    #[test]
    fn filter_by_case_id() {
        let (_dir, store) = test_store();

        store.emit(&EventBuilder::new(EventSource::Dataflow, "node.spawn").case_id("df_abc").build()).unwrap();
        store.emit(&EventBuilder::new(EventSource::Dataflow, "node.output").case_id("df_abc").build()).unwrap();
        store.emit(&EventBuilder::new(EventSource::Dataflow, "node.spawn").case_id("df_xyz").build()).unwrap();

        let results = store.query(&EventFilter { case_id: Some("df_abc".into()), ..Default::default() }).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn count_events() {
        let (_dir, store) = test_store();

        for i in 0..10 {
            store.emit(&EventBuilder::new(EventSource::Server, "http.request").case_id(format!("req_{}", i)).build()).unwrap();
        }

        let total = store.count(&EventFilter::default()).unwrap();
        assert_eq!(total, 10);

        let server_count = store.count(&EventFilter { source: Some("server".into()), ..Default::default() }).unwrap();
        assert_eq!(server_count, 10);
    }

    #[test]
    fn export_xes_format() {
        let (_dir, store) = test_store();

        store.emit(&EventBuilder::new(EventSource::Core, "node.install").case_id("s1").message("test").build()).unwrap();
        store.emit(&EventBuilder::new(EventSource::Core, "node.start").case_id("s1").build()).unwrap();

        let xes = store.export_xes(&EventFilter::default()).unwrap();
        assert!(xes.contains("xes.version"));
        assert!(xes.contains("concept:name"));
        assert!(xes.contains("node.install"));
        assert!(xes.contains("node.start"));
    }

    #[test]
    fn event_builder_attributes() {
        let event = EventBuilder::new(EventSource::Ci, "clippy.warn")
            .case_id("commit_abc123")
            .level(EventLevel::Warn)
            .message("unused variable")
            .attr("file", "src/main.rs")
            .attr("line", 42)
            .attr("severity", "warning")
            .build();

        assert_eq!(event.source, "ci");
        assert_eq!(event.level, "warn");

        let attrs: serde_json::Value = serde_json::from_str(event.attributes.as_ref().unwrap()).unwrap();
        assert_eq!(attrs["file"], "src/main.rs");
        assert_eq!(attrs["line"], 42);
    }
}
