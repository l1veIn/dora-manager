/// Transpile passes — each function performs one well-defined transformation
/// on the `DmGraph` intermediate representation.

use std::path::PathBuf;

use crate::node::{self, Node};

use super::context::TranspileContext;
use super::error::{DiagnosticKind, TranspileDiagnostic};
use super::model::{DmGraph, DmNode, ManagedNode};
use super::repo;

/// Node IDs reserved for DM built-in nodes.
/// Managed nodes with these names are treated specially by transpile.
const RESERVED_NODE_IDS: &[&str] = &["dm-panel", "dm-test-harness"];

fn is_reserved_node_id(id: &str) -> bool {
    RESERVED_NODE_IDS.contains(&id)
}

// ---------------------------------------------------------------------------
// Pass 1: Parse — YAML string → DmGraph
// ---------------------------------------------------------------------------

/// Parse raw YAML content into the typed `DmGraph` IR.
///
/// Each node is classified as `Panel`, `Managed`, or `External` based on the
/// presence of `node:` vs `path:` fields.
pub(crate) fn parse(content: &str) -> anyhow::Result<DmGraph> {
    let raw: serde_yaml::Value = serde_yaml::from_str(content)?;
    let raw_mapping = raw
        .as_mapping()
        .cloned()
        .unwrap_or_default();

    let mut extra_fields = raw_mapping.clone();
    extra_fields.remove(&serde_yaml::Value::String("nodes".to_string()));

    let mut nodes = Vec::new();
    if let Some(entries) = raw_mapping
        .get(&serde_yaml::Value::String("nodes".to_string()))
        .and_then(|n| n.as_sequence())
    {
        for entry in entries {
            let Some(mapping) = entry.as_mapping() else {
                continue;
            };

            let yaml_id = mapping
                .get(&serde_yaml::Value::String("id".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let node_field = mapping
                .get(&serde_yaml::Value::String("node".to_string()))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let path_field = mapping
                .get(&serde_yaml::Value::String("path".to_string()))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let node_id = node_field.as_deref().or(path_field.as_deref());

            // Build extra_fields: everything except id, node, path, config, widgets
            let mut node_extra = mapping.clone();
            for key in &["id", "node", "path", "config", "widgets"] {
                node_extra.remove(&serde_yaml::Value::String(key.to_string()));
            }

            // Extract widgets block (Panel-only, DM-specific)
            let widgets = mapping
                .get(&serde_yaml::Value::String("widgets".to_string()))
                .cloned();

            match node_id {
                Some(id) if is_reserved_node_id(id) => {
                    nodes.push(DmNode::Panel {
                        yaml_id,
                        extra_fields: node_extra,
                        widgets,
                    });
                }
                Some(id) if node_field.is_some() => {
                    let inline_config = mapping
                        .get(&serde_yaml::Value::String("config".to_string()))
                        .and_then(|v| serde_json::to_value(v).ok())
                        .unwrap_or(serde_json::json!({}));

                    // Preserve existing env from YAML
                    let existing_env = mapping
                        .get(&serde_yaml::Value::String("env".to_string()))
                        .and_then(|v| v.as_mapping().cloned())
                        .unwrap_or_default();

                    // Remove env from extra_fields since we manage it separately
                    node_extra.remove(&serde_yaml::Value::String("env".to_string()));

                    nodes.push(DmNode::Managed(ManagedNode {
                        yaml_id,
                        node_id: id.to_string(),
                        inline_config,
                        resolved_path: None,
                        merged_env: existing_env,
                        extra_fields: node_extra,
                    }));
                }
                _ => {
                    // External node or node without node:/path: — pass through as-is
                    nodes.push(DmNode::External {
                        _yaml_id: yaml_id,
                        raw: mapping.clone(),
                    });
                }
            }
        }
    }

    Ok(DmGraph {
        nodes,
        extra_fields,
    })
}

// ---------------------------------------------------------------------------
// Pass 1.5: Validate Reserved — check for conflicts
// ---------------------------------------------------------------------------

/// Emit diagnostics when managed nodes shadow reserved built-in names.
pub(crate) fn validate_reserved(
    ctx: &TranspileContext,
    graph: &DmGraph,
    diags: &mut Vec<TranspileDiagnostic>,
) {
    for node in &graph.nodes {
        let DmNode::Managed(managed) = node else {
            continue;
        };
        if is_reserved_node_id(&managed.node_id) {
            // The user installed a node with a reserved name, which will
            // never be transpiled as a managed node (parse classifies it
            // as Panel). Warn them.
            diags.push(TranspileDiagnostic {
                yaml_id: managed.yaml_id.clone(),
                node_id: managed.node_id.clone(),
                kind: DiagnosticKind::ReservedNodeId,
            });
        }
    }
    // Also check if a reserved name exists as an installed node — this could
    // cause confusion even though transpile handles it correctly.
    for reserved in RESERVED_NODE_IDS {
        if node::resolve_node_dir(ctx.home, reserved).is_some() {
            eprintln!(
                "[dm-core] warning: installed node '{}' shadows a reserved built-in name and will be ignored",
                reserved
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 1.6: Validate Port Schemas — check connection type compatibility
// ---------------------------------------------------------------------------

/// Validate that every wired connection between managed nodes has compatible
/// port schemas.
///
/// For each managed node's `inputs:` mapping, parse entries of the form
/// `input_port: source_node/source_output` and check that the source node's
/// output port schema is a subtype of this node's input port schema.
pub(crate) fn validate_port_schemas(
    ctx: &TranspileContext,
    graph: &DmGraph,
    diags: &mut Vec<TranspileDiagnostic>,
) {
    // Build a lookup: yaml_id → node_id (for managed nodes only)
    let mut yaml_to_node: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for node in &graph.nodes {
        if let DmNode::Managed(managed) = node {
            yaml_to_node.insert(&managed.yaml_id, &managed.node_id);
        }
    }

    for node in &graph.nodes {
        let DmNode::Managed(managed) = node else {
            continue;
        };

        // Extract `inputs:` from extra_fields
        let Some(inputs_val) = managed
            .extra_fields
            .get(&serde_yaml::Value::String("inputs".to_string()))
        else {
            continue;
        };
        let Some(inputs_map) = inputs_val.as_mapping() else {
            continue;
        };

        // Load this node's metadata (for input port schemas)
        let Some(this_meta) = load_node_meta(ctx, &managed.node_id) else {
            continue;
        };

        for (input_key, source_val) in inputs_map {
            let Some(input_port_id) = input_key.as_str() else {
                continue;
            };
            let Some(source_str) = source_val.as_str() else {
                continue;
            };

            // Parse "source_node/source_output" format
            let Some((source_yaml_id, source_output_id)) = source_str.split_once('/') else {
                continue; // dora built-in like "dora/timer/..." — skip
            };

            // Skip dora built-in sources
            if source_yaml_id == "dora" {
                continue;
            }

            // Find source node's node_id
            let Some(source_node_id) = yaml_to_node.get(source_yaml_id) else {
                continue; // External or Panel node — skip
            };

            // Load source node metadata
            let Some(source_meta) = load_node_meta(ctx, source_node_id) else {
                continue;
            };

            // Find port declarations in dm.json
            let source_port = source_meta
                .ports
                .iter()
                .find(|p| p.id == source_output_id);
            let input_port = this_meta.ports.iter().find(|p| p.id == input_port_id);

            // dynamic_ports: if port isn't declared in dm.json, skip silently
            if source_port.is_none() && source_meta.dynamic_ports {
                continue;
            }
            if input_port.is_none() && this_meta.dynamic_ports {
                continue;
            }

            let (Some(source_port), Some(input_port)) = (source_port, input_port) else {
                continue; // Port not declared in dm.json — skip
            };

            // If either side lacks a schema, skip validation silently.
            // Schema validation only triggers when BOTH sides declare schemas.
            let (Some(out_schema_val), Some(in_schema_val)) =
                (&source_port.schema, &input_port.schema)
            else {
                continue;
            };

            // Resolve schemas (handles $ref)
            let source_node_dir =
                node::resolve_node_dir(ctx.home, source_node_id).unwrap_or_default();
            let input_node_dir =
                node::resolve_node_dir(ctx.home, &managed.node_id).unwrap_or_default();

            let out_schema =
                match crate::node::schema::parse_schema(out_schema_val, &source_node_dir) {
                    Ok(s) => s,
                    Err(e) => {
                        diags.push(TranspileDiagnostic {
                            yaml_id: source_yaml_id.to_string(),
                            node_id: source_node_id.to_string(),
                            kind: DiagnosticKind::InvalidPortSchema {
                                port_id: source_output_id.to_string(),
                                reason: e.to_string(),
                            },
                        });
                        continue;
                    }
                };

            let in_schema =
                match crate::node::schema::parse_schema(in_schema_val, &input_node_dir) {
                    Ok(s) => s,
                    Err(e) => {
                        diags.push(TranspileDiagnostic {
                            yaml_id: managed.yaml_id.clone(),
                            node_id: managed.node_id.clone(),
                            kind: DiagnosticKind::InvalidPortSchema {
                                port_id: input_port_id.to_string(),
                                reason: e.to_string(),
                            },
                        });
                        continue;
                    }
                };

            // Check compatibility
            if let Err(e) = crate::node::schema::check_compatibility(&out_schema, &in_schema) {
                diags.push(TranspileDiagnostic {
                    yaml_id: managed.yaml_id.clone(),
                    node_id: managed.node_id.clone(),
                    kind: DiagnosticKind::IncompatiblePortSchema {
                        output_port: format!("{}/{}", source_yaml_id, source_output_id),
                        input_port: input_port_id.to_string(),
                        reason: e.to_string(),
                    },
                });
            }
        }
    }
}

/// Helper: load a node's metadata from dm.json.
fn load_node_meta(ctx: &TranspileContext, node_id: &str) -> Option<Node> {
    let meta_path = node::resolve_dm_json_path(ctx.home, node_id)?;
    let content = std::fs::read_to_string(&meta_path).ok()?;
    serde_json::from_str::<Node>(&content).ok()
}

// ---------------------------------------------------------------------------
// Pass 2: Resolve Paths — node: → path: (absolute)
// ---------------------------------------------------------------------------

/// Resolve managed node IDs to absolute executable paths via `dm.json`.
pub(crate) fn resolve_paths(
    ctx: &TranspileContext,
    graph: &mut DmGraph,
    diags: &mut Vec<TranspileDiagnostic>,
) {
    for node in &mut graph.nodes {
        let DmNode::Managed(managed) = node else {
            continue;
        };

        let Some(node_cache_dir) = node::resolve_node_dir(ctx.home, &managed.node_id) else {
            diags.push(TranspileDiagnostic {
                yaml_id: managed.yaml_id.clone(),
                node_id: managed.node_id.clone(),
                kind: DiagnosticKind::NodeNotInstalled,
            });
            continue;
        };
        let Some(meta_file_path) = node::resolve_dm_json_path(ctx.home, &managed.node_id) else {
            diags.push(TranspileDiagnostic {
                yaml_id: managed.yaml_id.clone(),
                node_id: managed.node_id.clone(),
                kind: DiagnosticKind::MetadataUnreadable {
                    path: node_cache_dir.join("dm.json"),
                },
            });
            continue;
        };

        if !node_cache_dir.exists() || !meta_file_path.exists() {
            diags.push(TranspileDiagnostic {
                yaml_id: managed.yaml_id.clone(),
                node_id: managed.node_id.clone(),
                kind: DiagnosticKind::MetadataUnreadable {
                    path: meta_file_path,
                },
            });
            continue;
        }

        let meta_content = std::fs::read_to_string(&meta_file_path).unwrap_or_default();
        let Ok(meta) = serde_json::from_str::<Node>(&meta_content) else {
            diags.push(TranspileDiagnostic {
                yaml_id: managed.yaml_id.clone(),
                node_id: managed.node_id.clone(),
                kind: DiagnosticKind::MetadataUnreadable {
                    path: meta_file_path,
                },
            });
            continue;
        };

        if meta.executable.is_empty() {
            diags.push(TranspileDiagnostic {
                yaml_id: managed.yaml_id.clone(),
                node_id: managed.node_id.clone(),
                kind: DiagnosticKind::MissingExecutable,
            });
        } else {
            let abs_exec = node_cache_dir.join(&meta.executable);
            managed.resolved_path = Some(abs_exec.display().to_string());
        }

        // Stash metadata for the config-merge pass (stored temporarily)
        managed
            .extra_fields
            .insert(
                serde_yaml::Value::String("__dm_meta_path".to_string()),
                serde_yaml::Value::String(meta_file_path.display().to_string()),
            );
    }
}

// ---------------------------------------------------------------------------
// Pass 3: Merge Config — config: four-layer merge → env:
// ---------------------------------------------------------------------------

/// Merge configuration from four sources (inline > flow > node > schema default)
/// and inject the result as environment variables.
pub(crate) fn merge_config(
    ctx: &TranspileContext,
    graph: &mut DmGraph,
    _diags: &mut Vec<TranspileDiagnostic>,
) {
    for node in &mut graph.nodes {
        let DmNode::Managed(managed) = node else {
            continue;
        };

        // Read the __dm_meta_path stashed by resolve_paths
        let meta_path_str = managed
            .extra_fields
            .get(&serde_yaml::Value::String("__dm_meta_path".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Clean up the temporary marker
        managed
            .extra_fields
            .remove(&serde_yaml::Value::String("__dm_meta_path".to_string()));

        let Some(meta_path_str) = meta_path_str else {
            continue;
        };

        let meta_content = std::fs::read_to_string(&meta_path_str).unwrap_or_default();
        let Ok(meta) = serde_json::from_str::<Node>(&meta_content) else {
            continue;
        };

        let Some(schema) = &meta.config_schema else {
            continue;
        };
        let Some(schema_obj) = schema.as_object() else {
            continue;
        };

        let config_defaults =
            node::get_node_config(ctx.home, &managed.node_id).unwrap_or(serde_json::json!({}));
        let flow_config_for_node =
            repo::select_flow_node_config(&ctx.flow_config, &managed.yaml_id, &managed.node_id);

        for (key, field_schema) in schema_obj {
            let Some(env_name) = field_schema.get("env").and_then(|e| e.as_str()) else {
                continue;
            };

            let value = managed
                .inline_config
                .get(key)
                .or_else(|| flow_config_for_node.get(key))
                .or_else(|| config_defaults.get(key))
                .or_else(|| field_schema.get("default"));

            if let Some(val) = value {
                let val_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                managed.merged_env.insert(
                    serde_yaml::Value::String(env_name.to_string()),
                    serde_yaml::Value::String(val_str),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 4: Inject Panel — dm-panel → dm binary + args
// ---------------------------------------------------------------------------

/// Transform Panel nodes: set `path` to the current `dm` binary and inject
/// `panel serve --run-id <id> --node-id <id>` as args.
pub(crate) fn inject_panel(ctx: &TranspileContext, graph: &mut DmGraph) {
    let dm_exe = resolve_dm_exe();

    for node in &mut graph.nodes {
        let DmNode::Panel {
            yaml_id,
            extra_fields,
            ..
        } = node
        else {
            continue;
        };

        // Skip test harness nodes — they are handled by inject_test_harness
        if yaml_id == "dm-test-harness" {
            continue;
        }

        extra_fields.insert(
            serde_yaml::Value::String("path".to_string()),
            serde_yaml::Value::String(dm_exe.display().to_string()),
        );
        extra_fields.insert(
            serde_yaml::Value::String("args".to_string()),
            serde_yaml::Value::String(format!(
                "panel serve --run-id {} --node-id {}",
                ctx.run_id, yaml_id
            )),
        );
    }
}

/// Transform test harness nodes: set `path` to the current `dm` binary and inject
/// `test harness-serve` with auto-trigger and output-ports args from env.
pub(crate) fn inject_test_harness(graph: &mut DmGraph) {
    let dm_exe = resolve_dm_exe();

    for node in &mut graph.nodes {
        let DmNode::Panel {
            yaml_id,
            extra_fields,
            ..
        } = node
        else {
            continue;
        };

        // Only match nodes that were parsed from `node: dm-test-harness`
        // The parser sees them as Panel because dm-test-harness is reserved.
        // We identify them by checking the env for DM_TEST_AUTO_TRIGGER.
        let is_harness = extra_fields
            .get(&serde_yaml::Value::String("env".to_string()))
            .and_then(|v| v.as_mapping())
            .and_then(|m| m.get(&serde_yaml::Value::String("DM_TEST_AUTO_TRIGGER".to_string())))
            .is_some();

        if !is_harness {
            continue;
        }

        let env_map = extra_fields
            .get(&serde_yaml::Value::String("env".to_string()))
            .and_then(|v| v.as_mapping())
            .cloned()
            .unwrap_or_default();

        let auto_trigger = env_map
            .get(&serde_yaml::Value::String("DM_TEST_AUTO_TRIGGER".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("false") == "true";

        let output_ports = env_map
            .get(&serde_yaml::Value::String("DM_TEST_OUTPUT_PORTS".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut args = format!("test harness-serve");
        if auto_trigger {
            args.push_str(" --auto-trigger");
        }
        if !output_ports.is_empty() {
            args.push_str(&format!(" --output-ports {}", output_ports));
        }

        extra_fields.insert(
            serde_yaml::Value::String("path".to_string()),
            serde_yaml::Value::String(dm_exe.display().to_string()),
        );
        extra_fields.insert(
            serde_yaml::Value::String("args".to_string()),
            serde_yaml::Value::String(args),
        );
        // Remove env block — harness gets config via args, not env
        extra_fields.remove(&serde_yaml::Value::String("env".to_string()));

        eprintln!(
            "[dm-core] injected test harness for node '{}'",
            yaml_id
        );
    }
}

/// Resolve the path to the `dm` binary (same directory as the current exe).
fn resolve_dm_exe() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let dir = exe.parent()?;
            let dm_path = dir.join("dm");
            if dm_path.exists() {
                Some(dm_path)
            } else {
                None
            }
        })
        .unwrap_or_else(|| PathBuf::from("dm"))
}

// ---------------------------------------------------------------------------
// Pass 5: Emit — DmGraph → serde_yaml::Value
// ---------------------------------------------------------------------------

/// Convert the typed IR back into a `serde_yaml::Value` suitable for
/// serialization and consumption by `dora start`.
pub(crate) fn emit(graph: &DmGraph) -> serde_yaml::Value {
    let mut root = graph.extra_fields.clone();

    let mut nodes_seq = Vec::new();
    for node in &graph.nodes {
        match node {
            DmNode::Panel {
                yaml_id,
                extra_fields,
                widgets: _, // widgets are NOT emitted into dora YAML
            } => {
                let mut m = serde_yaml::Mapping::new();
                m.insert(
                    serde_yaml::Value::String("id".to_string()),
                    serde_yaml::Value::String(yaml_id.clone()),
                );
                for (k, v) in extra_fields {
                    m.insert(k.clone(), v.clone());
                }
                nodes_seq.push(serde_yaml::Value::Mapping(m));
            }
            DmNode::Managed(managed) => {
                let mut m = serde_yaml::Mapping::new();
                m.insert(
                    serde_yaml::Value::String("id".to_string()),
                    serde_yaml::Value::String(managed.yaml_id.clone()),
                );

                if let Some(ref path) = managed.resolved_path {
                    m.insert(
                        serde_yaml::Value::String("path".to_string()),
                        serde_yaml::Value::String(path.clone()),
                    );
                } else {
                    // Unresolved: emit original `node:` so dora gives a clear error
                    m.insert(
                        serde_yaml::Value::String("node".to_string()),
                        serde_yaml::Value::String(managed.node_id.clone()),
                    );
                }

                if !managed.merged_env.is_empty() {
                    m.insert(
                        serde_yaml::Value::String("env".to_string()),
                        serde_yaml::Value::Mapping(managed.merged_env.clone()),
                    );
                }

                // Emit all extra fields (inputs, outputs, etc.)
                for (k, v) in &managed.extra_fields {
                    m.insert(k.clone(), v.clone());
                }

                nodes_seq.push(serde_yaml::Value::Mapping(m));
            }
            DmNode::External { _yaml_id: _, raw } => {
                nodes_seq.push(serde_yaml::Value::Mapping(raw.clone()));
            }
        }
    }

    root.insert(
        serde_yaml::Value::String("nodes".to_string()),
        serde_yaml::Value::Sequence(nodes_seq),
    );

    serde_yaml::Value::Mapping(root)
}

// ---------------------------------------------------------------------------
// Extract: widgets config → JSON (for external storage)
// ---------------------------------------------------------------------------

/// Collect widget definitions from all Panel nodes into a single JSON object.
///
/// Returns `None` if no Panel node declares widgets.
/// Output shape: `{ "<output_id>": { "default": ..., "x-widget": { ... } }, ... }`
pub(crate) fn extract_widgets(graph: &DmGraph) -> Option<serde_json::Value> {
    let mut all_widgets = serde_json::Map::new();

    for node in &graph.nodes {
        let DmNode::Panel { widgets, .. } = node else {
            continue;
        };
        let Some(widgets_val) = widgets else {
            continue;
        };
        // Convert YAML value → JSON value
        if let Ok(json) = serde_json::to_value(widgets_val) {
            if let serde_json::Value::Object(map) = json {
                all_widgets.extend(map);
            }
        }
    }

    if all_widgets.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(all_widgets))
    }
}
