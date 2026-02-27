use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use uuid::Uuid;

use super::{Event, EventBuilder, EventLevel, EventSource, EventStore};

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
