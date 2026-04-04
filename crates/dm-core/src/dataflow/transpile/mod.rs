/// Transpile module — transforms DM-flavoured YAML into standard dora-rs YAML.
///
/// The pipeline:
/// 1. **parse**                  — YAML text  →  typed `DmGraph` IR
/// 2. **validate_reserved**      — check for reserved node ID conflicts
/// 3. **resolve_paths**          — `node:` → absolute `path:` via `dm.json`
/// 4. **validate_port_schemas**  — check port schema compatibility
/// 5. **merge_config**           — four-layer config merge → `env:`
/// 6. **emit**                   — `DmGraph` → `serde_yaml::Value`
mod context;
mod error;
mod model;
mod passes;

use std::path::Path;

use anyhow::{Context, Result};

use crate::events::{EventSource, OperationEvent};

use context::TranspileContext;


/// Result of a transpilation, containing the dora-compatible YAML.
#[derive(Debug)]
pub struct TranspileResult {
    /// Standard dora `Descriptor` YAML ready for `dora start`.
    pub yaml: serde_yaml::Value,
}

/// Transpile a DM graph YAML, generating a fresh run-id automatically.
pub fn transpile_graph(home: &Path, yaml_path: &Path) -> Result<TranspileResult> {
    transpile_graph_for_run(home, yaml_path, &uuid::Uuid::new_v4().to_string())
}

/// Transpile a DM graph YAML with an explicit run-id.
pub fn transpile_graph_for_run(
    home: &Path,
    yaml_path: &Path,
    _run_id: &str,
) -> Result<TranspileResult> {
    let op = OperationEvent::new(home, EventSource::Dataflow, "dataflow.transpile")
        .attr("path", yaml_path.display().to_string());
    op.emit_start();

    let result = (|| {
        let content = std::fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read graph yaml at {}", yaml_path.display()))?;

        let ctx = TranspileContext {
            home,
            run_id: _run_id,
        };
        let mut diags = Vec::new();

        // Parse
        let mut graph = passes::parse(&content)
            .with_context(|| format!("Failed to parse yaml at {}", yaml_path.display()))?;

        // Validate
        passes::validate_reserved(&ctx, &graph, &mut diags);

        // Transform
        passes::resolve_paths(&ctx, &mut graph, &mut diags);
        passes::validate_port_schemas(&ctx, &graph, &mut diags);
        passes::merge_config(&ctx, &mut graph, &mut diags);
        passes::inject_runtime_env(&ctx, &mut graph);

        // Log diagnostics as warnings
        for d in &diags {
            eprintln!("[dm-core] transpile warning: {}", d);
        }

        // Emit
        Ok(TranspileResult { yaml: passes::emit(&graph) })
    })();

    op.emit_result(&result);
    result
}
