use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataflowMeta {
    pub name: String,
    pub filename: String,
    pub modified_at: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataflowHistoryEntry {
    pub version: String,
    pub modified_at: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FlowMeta {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataflowExecutableStatus {
    #[default]
    Ready,
    MissingNodes,
    InvalidYaml,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataflowExecutableSummary {
    #[serde(default)]
    pub status: DataflowExecutableStatus,
    #[serde(default)]
    pub can_run: bool,
    #[serde(default)]
    pub can_configure: bool,
    #[serde(default)]
    pub declared_node_count: usize,
    #[serde(default)]
    pub resolved_node_count: usize,
    #[serde(default)]
    pub missing_node_count: usize,
    #[serde(default)]
    pub missing_nodes: Vec<String>,
    #[serde(default)]
    pub invalid_yaml: bool,
    #[serde(default)]
    pub requires_media_backend: bool,
    #[serde(default)]
    pub media_node_count: usize,
    #[serde(default)]
    pub media_nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataflowNodeResolution {
    #[serde(default)]
    pub yaml_id: String,
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default)]
    pub configurable: bool,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataflowExecutableDetail {
    #[serde(flatten)]
    pub summary: DataflowExecutableSummary,
    #[serde(default)]
    pub nodes: Vec<DataflowNodeResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowListEntry {
    #[serde(flatten)]
    pub file: DataflowMeta,
    pub meta: FlowMeta,
    pub executable: DataflowExecutableSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowProject {
    pub name: String,
    pub yaml: String,
    pub meta: FlowMeta,
    pub executable: DataflowExecutableSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowImportSuccess {
    pub name: String,
    pub executable: DataflowExecutableSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowImportFailure {
    pub source: String,
    pub name: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowImportReport {
    pub imported: Vec<DataflowImportSuccess>,
    pub failed: Vec<DataflowImportFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedConfigField {
    pub schema: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_value: Option<serde_json::Value>,
    pub effective_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedConfigNode {
    pub yaml_id: String,
    pub node_id: String,
    pub resolved: bool,
    pub configurable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_reason: Option<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, AggregatedConfigField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowConfigAggregation {
    pub executable: DataflowExecutableSummary,
    #[serde(default)]
    pub nodes: Vec<AggregatedConfigNode>,
}
