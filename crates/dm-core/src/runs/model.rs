use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    #[default]
    Running,
    Succeeded,
    Stopped,
    Failed,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunSource {
    #[default]
    Unknown,
    Cli,
    Server,
    Web,
}

impl RunSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Cli => "cli",
            Self::Server => "server",
            Self::Web => "web",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    #[default]
    Completed,
    StoppedByUser,
    StartFailed,
    NodeFailed,
    RuntimeLost,
    RuntimeStopped,
}

impl TerminationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::StoppedByUser => "stopped_by_user",
            Self::StartFailed => "start_failed",
            Self::NodeFailed => "node_failed",
            Self::RuntimeLost => "runtime_lost",
            Self::RuntimeStopped => "runtime_stopped",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogSyncState {
    #[default]
    Pending,
    Synced,
}

impl LogSyncState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Synced => "synced",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunOutcome {
    #[serde(default)]
    pub status: RunStatus,
    #[serde(default)]
    pub termination_reason: Option<TerminationReason>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTranspileMetadata {
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub panel_node_ids: Vec<String>,
    #[serde(default)]
    pub resolved_node_paths: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunLogSync {
    #[serde(default)]
    pub state: LogSyncState,
    #[serde(default)]
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RunInstance {
    pub schema_version: u32,
    pub run_id: String,
    pub dora_uuid: Option<String>,
    pub dataflow_name: String,
    pub dataflow_hash: String,
    pub source: RunSource,
    pub has_panel: bool,
    pub status: RunStatus,
    pub termination_reason: Option<TerminationReason>,
    pub failure_reason: Option<String>,
    pub failure_node: Option<String>,
    pub failure_message: Option<String>,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub runtime_observed_at: Option<String>,
    pub exit_code: Option<i32>,
    pub outcome: RunOutcome,
    pub transpile: RunTranspileMetadata,
    pub log_sync: RunLogSync,
    pub node_count_expected: u32,
    pub node_count_observed: u32,
    pub nodes_expected: Vec<String>,
    #[serde(default, alias = "nodes")]
    pub nodes_observed: Vec<String>,
}

impl Default for RunInstance {
    fn default() -> Self {
        Self {
            schema_version: 1,
            run_id: String::new(),
            dora_uuid: None,
            dataflow_name: String::new(),
            dataflow_hash: String::new(),
            source: RunSource::Unknown,
            has_panel: false,
            status: RunStatus::Running,
            termination_reason: None,
            failure_reason: None,
            failure_node: None,
            failure_message: None,
            started_at: String::new(),
            stopped_at: None,
            runtime_observed_at: None,
            exit_code: None,
            outcome: RunOutcome::default(),
            transpile: RunTranspileMetadata::default(),
            log_sync: RunLogSync::default(),
            node_count_expected: 0,
            node_count_observed: 0,
            nodes_expected: Vec::new(),
            nodes_observed: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunResult {
    pub run: RunInstance,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartConflictStrategy {
    Fail,
    StopAndRestart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: String,
    pub name: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub source: String,
    pub has_panel: bool,
    pub node_count: u32,
    pub status: String,
    pub termination_reason: Option<String>,
    pub outcome_summary: String,
    pub dora_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<RunMetrics>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetrics {
    pub cpu: Option<f64>,
    pub memory_mb: Option<f64>,
    pub nodes: Vec<NodeMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub id: String,
    pub status: String,
    pub pid: Option<String>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunNode {
    pub id: String,
    pub log_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLogChunk {
    pub run_id: String,
    pub node_id: String,
    pub offset: u64,
    pub next_offset: u64,
    pub content: String,
    pub finished: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDetail {
    #[serde(flatten)]
    pub summary: RunSummary,
    pub nodes: Vec<RunNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedRuns {
    pub runs: Vec<RunSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunListFilter {
    pub status: Option<String>,
    pub search: Option<String>,
    pub has_panel: Option<bool>,
}
