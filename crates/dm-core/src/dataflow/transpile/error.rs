use std::fmt;
use std::path::PathBuf;

/// A single diagnostic emitted during transpilation.
///
/// Diagnostics are collected (not short-circuited) so that the user sees
/// all issues at once rather than fixing them one by one.
#[derive(Debug, Clone)]
pub struct TranspileDiagnostic {
    pub yaml_id: String,
    pub node_id: String,
    pub kind: DiagnosticKind,
}

#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    /// The node directory does not exist in `~/.dm/nodes/`.
    NodeNotInstalled,
    /// `dm.json` could not be found or parsed.
    MetadataUnreadable { path: PathBuf },
    /// `dm.json` exists but `executable` field is empty.
    MissingExecutable,
    /// A managed node ID conflicts with a reserved built-in name.
    ReservedNodeId,
}

impl fmt::Display for TranspileDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let detail = match &self.kind {
            DiagnosticKind::NodeNotInstalled => {
                "not installed".to_string()
            }
            DiagnosticKind::MetadataUnreadable { path } => {
                format!("metadata unreadable at {}", path.display())
            }
            DiagnosticKind::MissingExecutable => {
                "dm.json has empty executable field".to_string()
            }
            DiagnosticKind::ReservedNodeId => {
                "conflicts with a reserved built-in node name".to_string()
            }
        };
        write!(
            f,
            "node \"{}\" (id: {}): {}",
            self.yaml_id, self.node_id, detail
        )
    }
}
