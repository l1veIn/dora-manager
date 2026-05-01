use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PushMessageRequest {
    pub from: String,
    pub tag: String,
    pub payload: serde_json::Value,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct ListMessagesParams {
    pub after_seq: Option<i64>,
    pub before_seq: Option<i64>,
    #[serde(rename = "from")]
    pub from_filter: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<usize>,
    pub desc: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NodeWsParams {
    pub since: Option<i64>,
}

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
pub struct StreamViewer {
    pub preferred: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webrtc_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hls_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StreamDescriptor {
    pub stream_id: String,
    pub from: String,
    pub kind: String,
    pub label: String,
    pub path: String,
    pub live: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
    pub seq: i64,
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer: Option<StreamViewer>,
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
