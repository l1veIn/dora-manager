use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A registered function, parsed from `service.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entry: String, // e.g. "service.py"
    #[serde(default)]
    pub methods: Vec<FunctionMethod>,
    #[serde(default)]
    pub runtime: FunctionRuntime,
    /// Absolute path to the function directory (set at discovery time, not in JSON).
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMethod {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub events: Vec<String>, // event names this method can emit (e.g. "on_token")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRuntime {
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

fn default_max_workers() -> usize {
    3
}

fn default_idle_timeout() -> u64 {
    300
}

impl Default for FunctionRuntime {
    fn default() -> Self {
        Self {
            max_workers: default_max_workers(),
            idle_timeout_secs: default_idle_timeout(),
        }
    }
}

// ── Request / Response types ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeRequest {
    pub method: String,
    pub input: serde_json::Value,
    #[serde(default)]
    pub subscribe: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeResponse {
    pub output: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionEvent {
    pub name: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub uptime_secs: u64,
    pub functions: usize,
    pub workers: usize,
}
