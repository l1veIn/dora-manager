use chrono::Utc;
use serde::Serialize;

use super::{Event, EventLevel, EventSource};

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
