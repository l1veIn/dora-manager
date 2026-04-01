use std::path::Path;

/// Read-only context shared by all transpile passes.
pub(crate) struct TranspileContext<'a> {
    /// DM home directory (e.g. `~/.dm`).
    pub home: &'a Path,
    /// Unique run identifier for generic runtime env injection.
    pub run_id: &'a str,
    /// Dataflow-level config loaded from `config.json`.
    pub flow_config: serde_json::Value,
}
