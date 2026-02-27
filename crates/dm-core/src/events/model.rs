use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    pub id: i64,
    pub timestamp: String,
    pub case_id: String,
    pub activity: String,
    pub source: String,
    pub level: String,
    pub node_id: Option<String>,
    pub message: Option<String>,
    pub attributes: Option<String>,
}

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
    pub offset: Option<i64>,
    pub search: Option<String>,
}
