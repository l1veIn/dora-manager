use serde::{Deserialize, Serialize};

// ─── Environment Check ───

/// A single environment prerequisite check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvItem {
    pub name: String,
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub suggestion: Option<String>,
}

/// Full environment health report returned by `doctor()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub python: EnvItem,
    pub uv: EnvItem,
    pub rust: EnvItem,
    pub installed_versions: Vec<InstalledVersion>,
    pub active_version: Option<String>,
    pub active_binary_ok: bool,
    pub all_ok: bool,
}

// ─── Version Management ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub version: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableVersion {
    pub tag: String,
    pub installed: bool,
}

/// Report returned by `versions()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionsReport {
    pub installed: Vec<InstalledVersion>,
    pub available: Vec<AvailableVersion>,
}

// ─── Install ───

/// Install progress phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallPhase {
    Fetching,
    Downloading { bytes_done: u64, bytes_total: u64 },
    Extracting,
    Building,
    Done,
}

/// Progress message sent during installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub message: String,
}

/// Method used to install dora
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallMethod {
    Binary,
    Source,
}

/// Result of a successful install
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub version: String,
    pub method: InstallMethod,
    pub set_active: bool,
}

// ─── Runtime ───

/// Result of up/down commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeResult {
    pub success: bool,
    pub message: String,
}

/// Status report returned by `status()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub active_version: Option<String>,
    pub actual_version: Option<String>,
    pub dm_home: String,
    pub runtime_running: bool,
    pub runtime_output: String,
    pub dataflows: Vec<String>,
}

// ─── Setup ───

/// Report returned by `setup()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupReport {
    pub python_installed: bool,
    pub uv_installed: bool,
    pub dora_installed: bool,
    pub dora_version: Option<String>,
}
