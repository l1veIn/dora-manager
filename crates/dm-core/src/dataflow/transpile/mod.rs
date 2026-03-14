/// Transpile module — transforms DM-flavoured YAML into standard dora-rs YAML.
///
/// The pipeline:
/// 1. **parse**              — YAML text  →  typed `DmGraph` IR
/// 2. **validate_reserved**  — check for reserved node ID conflicts
/// 3. **resolve_paths**      — `node:` → absolute `path:` via `dm.json`
/// 4. **merge_config**       — four-layer config merge → `env:`
/// 5. **inject_panel**       — `dm-panel` → current `dm` binary + args
/// 6. **extract_widgets**    — collect panel widget definitions → JSON
/// 7. **emit**               — `DmGraph` → `serde_yaml::Value`

mod context;
mod error;
mod model;
mod passes;

use std::path::Path;

use anyhow::{Context, Result};

use crate::events::{EventSource, OperationEvent};

use context::TranspileContext;


use super::repo;

/// Result of a transpilation, containing both the dora-compatible YAML
/// and any DM-specific metadata extracted during the process.
#[derive(Debug)]
pub struct TranspileResult {
    /// Standard dora `Descriptor` YAML ready for `dora start`.
    pub yaml: serde_yaml::Value,
    /// Widget definitions extracted from `widgets:` fields on panel nodes.
    /// `None` if no panel node declares widgets.
    pub widgets: Option<serde_json::Value>,
}

/// Transpile a DM graph YAML, generating a fresh run-id automatically.
pub fn transpile_graph(home: &Path, yaml_path: &Path) -> Result<TranspileResult> {
    transpile_graph_for_run(home, yaml_path, &uuid::Uuid::new_v4().to_string())
}

/// Transpile a DM graph YAML with an explicit run-id.
pub fn transpile_graph_for_run(
    home: &Path,
    yaml_path: &Path,
    run_id: &str,
) -> Result<TranspileResult> {
    let op = OperationEvent::new(home, EventSource::Dataflow, "dataflow.transpile")
        .attr("path", yaml_path.display().to_string());
    op.emit_start();

    let result = (|| {
        let content = std::fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read graph yaml at {}", yaml_path.display()))?;
        let flow_config =
            repo::load_flow_config_for_yaml(home, yaml_path).unwrap_or(serde_json::json!({}));

        let ctx = TranspileContext {
            home,
            run_id,
            flow_config,
        };
        let mut diags = Vec::new();

        // Parse
        let mut graph = passes::parse(&content)
            .with_context(|| format!("Failed to parse yaml at {}", yaml_path.display()))?;

        // Validate
        passes::validate_reserved(&ctx, &graph, &mut diags);

        // Transform
        passes::resolve_paths(&ctx, &mut graph, &mut diags);
        passes::merge_config(&ctx, &mut graph, &mut diags);
        passes::inject_panel(&ctx, &mut graph);
        passes::inject_test_harness(&mut graph);

        // Extract DM-specific metadata
        let widgets = passes::extract_widgets(&graph);

        // Log diagnostics as warnings
        for d in &diags {
            eprintln!("[dm-core] transpile warning: {}", d);
        }

        // Emit
        Ok(TranspileResult {
            yaml: passes::emit(&graph),
            widgets,
        })
    })();

    op.emit_result(&result);
    result
}
