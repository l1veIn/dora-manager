use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use dm_core::events::EventStore;

#[derive(Clone)]
pub struct AppState {
    pub home: Arc<std::path::PathBuf>,
    pub events: Arc<EventStore>,
    pub messages: broadcast::Sender<MessageNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageNotification {
    pub run_id: String,
    pub seq: i64,
    pub from: String,
    pub tag: String,
}
