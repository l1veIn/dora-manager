# Refactoring Plan: Remove Panel Coupling from dm-core

> **⚠️ Execute on a NEW branch. Create from main before starting.**
>
> ```bash
> git checkout main && git pull
> git checkout -b refactor/remove-panel-coupling
> ```

## Background

dm-core currently has deep coupling with `dm-panel` and `dm-test-harness` — two specific node types that receive special treatment in the transpiler, run model, and service layer. This violates the principle that **dm-core should be node-agnostic**. The core's job is to manage dataflow lifecycle, node start/stop, and data routing. It should not know about any specific node.

DM is unreleased. There is no backward compatibility concern. **Delete everything panel-related. No deprecation, no migration path.**

## Phase 1: dm-core Transpiler Cleanup

### 1.1 `crates/dm-core/src/dataflow/transpile/model.rs`

Delete the `DmNode::Panel` variant entirely. Nodes that were previously parsed as `Panel` (because their ID matched `RESERVED_NODE_IDS`) should now follow the standard `Managed` or `External` classification path.

Before:
```rust
pub(crate) enum DmNode {
    Panel { yaml_id, extra_fields, widgets },
    Managed(ManagedNode),
    External { _yaml_id, raw },
}
```

After:
```rust
pub(crate) enum DmNode {
    Managed(ManagedNode),
    External { _yaml_id, raw },
}
```

### 1.2 `crates/dm-core/src/dataflow/transpile/passes.rs`

1. Delete `RESERVED_NODE_IDS` constant (line 14)
2. Delete `is_reserved_node_id()` function (line 16-18)
3. In `parse()`: remove the branch that classifies reserved IDs as `Panel`. These nodes should go through the normal Managed/External classification based on `node:` vs `path:` fields
4. Delete `inject_panel()` function entirely (~line 470-502) — this injected `path` and `args` for panel nodes
5. Delete `inject_test_harness()` function entirely (~line 505-590) — this injected harness-specific args
6. In the main `transpile()` function: remove calls to `inject_panel()` and `inject_test_harness()` from the pass pipeline
7. Also search for and remove any `widgets` extraction logic that was Panel-specific

### 1.3 `crates/dm-core/src/dataflow/transpile/` other files

Check `mod.rs`, `context.rs`, `repo.rs` in the transpile directory for any `panel` or `widget` references and clean up.

### 1.4 `crates/dm-core/src/dataflow/inspect.rs`

Around line 53, there is a hardcoded skip:
```rust
if node_id == "dm-panel" || node_id == "dm-test-harness" {
```
Delete this condition. All nodes should be treated equally in inspection.

---

## Phase 2: dm-core Runs Model Cleanup

### 2.1 `crates/dm-core/src/runs/model.rs`

Delete the following fields:

| Struct | Field to delete |
|--------|----------------|
| `RunInstance` | `has_panel: bool` (~line 130) |
| `RunSummary` | `has_panel: bool` (~line 200) |
| `RunListFilter` | `has_panel: Option<bool>` (~line 262) |
| `RunTranspileMetadata` | `panel_node_ids: Vec<String>` (~line 108) |

Also delete `has_panel: false` from the `Default` impl (~line 159).

### 2.2 `crates/dm-core/src/runs/graph.rs`

In `build_transpile_metadata()`:
- Delete `panel_node_ids` variable declaration (line 14)
- Delete the entire `is_panel` detection block (lines 39-49)
- Delete `panel_node_ids.sort()` and `panel_node_ids.dedup()` (lines 53-54)
- Remove `panel_node_ids` from the returned struct (line 58)

### 2.3 `crates/dm-core/src/runs/service_start.rs`

Delete the entire widgets + PanelStore pre-seeding block (approximately lines 148-171):
```rust
// Write panel widgets config and pre-seed default commands
if let Some(ref widgets) = transpile_result.widgets {
    ...
}
```

Delete `has_panel` computation and usage:
```rust
let has_panel = !transpile.panel_node_ids.is_empty();  // DELETE
...
has_panel,  // DELETE from RunInstance constructor
```

### 2.4 `crates/dm-core/src/runs/service_query.rs`

- In `to_summary()`: delete `has_panel: run.has_panel` mapping (~line 117)
- In `apply_run_list_filter()`: delete the `has_panel` filter block (~lines 145-148)

### 2.5 Delete Panel submodule

**Delete the entire directory**: `crates/dm-core/src/runs/panel/`
- `mod.rs` (includes tests)
- `model.rs`
- `store.rs`

In `crates/dm-core/src/runs/mod.rs`:
- Delete `pub mod panel;`
- Delete `run_panel_dir` from the re-export list

### 2.6 `crates/dm-core/src/runs/repo.rs`

Delete the `run_panel_dir()` function:
```rust
pub fn run_panel_dir(home: &Path, run_id: &str) -> PathBuf {
    run_dir(home, run_id).join("panel")
}
```

### 2.7 `crates/dm-core/src/types.rs`

Delete `has_panel: bool` field (~line 112).

### 2.8 `crates/dm-core/src/api/runtime.rs`

Delete `has_panel: run.has_panel` mapping (~line 127).

### 2.9 `crates/dm-core/src/tests/tests_types.rs`

Delete all `has_panel: true` and `has_panel: false` field assignments (~lines 200, 212).

### 2.10 Test dataflows

Search `tests/dataflows/*.yml` for any `dm-panel` node declarations, related `inputs:` wiring, and `widgets:` blocks. Remove those nodes and their connections from the test YAML files.

```bash
grep -rl "dm-panel" tests/
```

---

## Phase 3: dm-server Cleanup

### 3.1 Delete panel handlers

**Delete files**:
- `crates/dm-server/src/handlers/panel.rs`
- `crates/dm-server/src/handlers/panel_ws.rs`

### 3.2 `crates/dm-server/src/handlers/mod.rs`

- Delete `mod panel;` and `mod panel_ws;`
- Delete all `pub use panel::*` and `pub use panel_ws::*` re-exports

### 3.3 `crates/dm-server/src/main.rs`

Delete all panel-related routes. Search for routes containing `panel`:
```rust
.route("/api/runs/{id}/panel/assets", ...)
.route("/api/runs/{id}/panel/files/*path", ...)
.route("/api/runs/{id}/panel/command", ...)
.route("/api/runs/{id}/panel/widgets", ...)
.route("/api/runs/{id}/panel/latest", ...)
.route("/api/runs/{id}/panel/ws", ...)
```

### 3.4 `crates/dm-server/src/handlers/runs.rs`

- Delete `has_panel: Option<bool>` from `ListRunsParams` (~line 16)
- Delete `has_panel` from `RunListFilter` construction (~line 52, 73)

### 3.5 `crates/dm-server/src/handlers/run_ws.rs`

Delete the `dm-panel` filter in metrics processing:
```rust
metrics.nodes.retain(|n| n.id != "dm-panel");  // DELETE this line
```

### 3.6 `crates/dm-server/src/tests.rs`

Delete all panel-related test functions and helpers:
- `setup_panel_run()` helper function
- `query_panel_assets_rejects_run_without_panel` test
- `query_panel_assets_returns_results_for_panel_run` test
- `send_panel_command_*` tests
- `serve_asset_file_*` tests
- Any remaining `has_panel` field assignments in test setup code

---

## Phase 4: dm-cli Cleanup

### 4.1 Delete builtin panel/harness

**Delete files**:
- `crates/dm-cli/src/builtin/panel.rs`
- `crates/dm-cli/src/builtin/test_harness.rs`

### 4.2 `crates/dm-cli/src/builtin/mod.rs`

Delete `pub mod panel;` and `pub mod test_harness;`. If the module is now empty, delete the entire `builtin/` directory and remove `mod builtin;` from the crate root.

### 4.3 `crates/dm-cli/src/main.rs`

- Delete `PanelCommands` enum (~line 159)
- Delete `Commands::Panel(PanelCommands)` variant (~line 98)
- Delete the `Commands::Panel(command) => match command { ... }` block (~line 306-315)
- Delete test harness related subcommands and their match arms

### 4.4 `crates/dm-cli/src/display.rs`

Delete "Panel" column from the runs table:
- Remove `"Panel"` from the header array (~line 128)
- Remove `if item.has_panel { "yes" } else { "no" }` row (~line 137)

### 4.5 `crates/dm-cli/src/cmd/test.rs`

Review this file for references to `dm-test-harness` or `builtin::test_harness`. If the entire `dm test` command depends on the harness, delete the whole file and remove the subcommand registration.

---

## Phase 5: Web Frontend Cleanup

### 5.1 Delete panel components

**Delete files**:
- `web/src/routes/runs/[id]/PanelPane.svelte`
- `web/src/routes/runs/[id]/PanelControls.svelte`
- `web/src/routes/runs/[id]/PanelMessage.svelte`

**Delete directory**:
- `web/src/routes/runs/[id]/panel/` (all display + control widgets)

### 5.2 `web/src/routes/runs/[id]/+page.svelte`

Remove PanelPane import and rendering.

### 5.3 `web/src/routes/runs/[id]/RunSummaryCard.svelte`

Remove "Panel Present" badge.

### 5.4 `web/src/routes/runs/[id]/RunHeader.svelte`

Remove any `has_panel` conditional logic.

### 5.5 `web/src/routes/runs/+page.svelte` (list page)

Remove `has_panel` filter option.

### 5.6 `web/src/routes/runs/[id]/types.ts`

Remove panel-related type definitions.

---

## Verification

Run in order, fix any errors before proceeding:

```bash
# 1. Rust workspace
cargo check --workspace
cargo test --workspace

# 2. Frontend
cd web && npm run check
```

### Smoke test
- A dataflow without panel nodes should `dm start` / `dm stop` normally
- `dm list` should display runs table without "Panel" column
- Web runs list page and run detail page should load without errors

---

## What NOT to do

- Do NOT try to make `dm-panel` work as a managed node in this refactoring
- Do NOT implement `dm-store` or the new Panel IPC architecture
- Do NOT worry about backward compatibility with existing run.json files on disk
- Focus ONLY on deleting panel coupling and making everything compile clean
