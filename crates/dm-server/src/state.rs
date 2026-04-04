use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use dm_core::events::EventStore;
use crate::services::input::InputEvent;

#[derive(Clone)]
pub struct AppState {
    pub home: Arc<std::path::PathBuf>,
    pub events: Arc<EventStore>,
    pub interaction_events: broadcast::Sender<InteractionNotification>,
    pub input_events: broadcast::Sender<InputEventNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionNotification {
    pub event: String,
    pub run_id: String,
    pub source_id: Option<String>,
    pub seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEventNotification {
    pub run_id: String,
    pub event: InputEvent,
}
