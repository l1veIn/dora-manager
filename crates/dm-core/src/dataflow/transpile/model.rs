/// Typed intermediate representation for the transpile pipeline.
///
/// Instead of manipulating raw `serde_yaml::Value` trees with string keys,
/// the transpiler parses each node into one of three strongly-typed variants
/// and converts them back to YAML only in the final emit pass.
///
/// A parsed DM graph — the core IR that every pass operates on.
pub(crate) struct DmGraph {
    pub nodes: Vec<DmNode>,
    /// Top-level fields other than `nodes` (e.g. `communication`, `deploy`, `debug`).
    /// Preserved verbatim so that unknown/future dora fields are never dropped.
    pub extra_fields: serde_yaml::Mapping,
}

/// One node in the DM graph, classified by its source type.
pub(crate) enum DmNode {
    /// A managed node installed in `~/.dm/nodes/<node_id>/`.
    Managed(ManagedNode),
    /// An external node specified by `path:` — not managed by DM.
    External {
        _yaml_id: String,
        raw: serde_yaml::Mapping,
    },
}

/// Details for a managed node that the resolve and config passes populate.
pub(crate) struct ManagedNode {
    pub yaml_id: String,
    pub node_id: String,
    /// Inline `config:` block from the YAML, if any.
    pub inline_config: serde_json::Value,
    /// Resolved absolute path to the executable (populated by resolve pass).
    pub resolved_path: Option<String>,
    /// Merged environment variables (populated by config-merge pass).
    pub merged_env: serde_yaml::Mapping,
    /// All other YAML fields (inputs, outputs, etc.) preserved verbatim.
    pub extra_fields: serde_yaml::Mapping,
}
