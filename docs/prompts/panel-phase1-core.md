# Phase 1: dm-core panel module

## Context

Read `docs/dm-panel-design.md` for the full architecture.

## Task

Create `crates/dm-core/src/panel/` module with pure storage + query logic. No networking.

### Files to create

#### `crates/dm-core/src/panel/mod.rs`

Public API, re-exports, module-level tests.

#### `crates/dm-core/src/panel/model.rs`

```rust
pub struct Asset { seq, input_id, timestamp, type_, storage, path, data }
pub struct AssetFilter { since_seq, input_id, limit }
pub struct PaginatedAssets { assets, total }
pub struct OutputCommand { seq, output_id, value, timestamp }
pub struct PanelSession { run_id, asset_count, command_count, disk_size_bytes, last_modified }
```

All derive `Debug, Clone, Serialize, Deserialize`.

#### `crates/dm-core/src/panel/store.rs`

`PanelStore` manages `~/.dm/panel/<run_id>/index.db` + asset files.

```rust
impl PanelStore {
    pub fn open(home: &Path, run_id: &str) -> Result<Self>
    // Creates dir + index.db with WAL mode
    // Two tables: assets + commands (see design doc schema)

    pub fn write_asset(&self, input_id: &str, type_hint: &str, data: &[u8]) -> Result<i64>
    // Classify: if text/json → inline (data column), else → file
    // File path: <input_id>/<seq_number>.<ext>
    // INSERT INTO assets

    pub fn query_assets(&self, filter: &AssetFilter) -> Result<PaginatedAssets>
    // SELECT with seq > filter.since_seq, optional input_id, LIMIT

    pub fn write_command(&self, output_id: &str, value: &str) -> Result<i64>
    // INSERT INTO commands

    pub fn poll_commands(&self, since_seq: &mut i64) -> Result<Vec<OutputCommand>>
    // SELECT WHERE seq > since_seq, update since_seq

    pub fn list_sessions(home: &Path) -> Result<Vec<PanelSession>>
    // Scan ~/.dm/panel/*/, stat each dir

    pub fn clean(home: &Path, keep: usize) -> Result<u32>
    // Delete oldest sessions, integrate with runs::clean_runs
}
```

### Wire up

In `crates/dm-core/src/lib.rs`, add:

```rust
pub mod panel;
```

### Tests

Add tests in `panel/mod.rs`:
- `write_and_query_assets`: write text + binary, query back
- `write_and_poll_commands`: write commands, poll, verify seq tracking
- `list_sessions`: create multiple sessions, list
- `clean_sessions`: create 5 sessions, clean keep=2

### Reference

Follow patterns from existing `events/` module (same crate):
- `events/store.rs` for SQLite patterns (WAL, Mutex<Connection>, query builder)
- `events/model.rs` for model struct patterns
- `runs.rs` for filesystem scanning patterns

### Dependencies

No new dependencies needed. Uses existing: `rusqlite`, `serde`, `serde_json`, `anyhow`, `chrono`.

### Verification

```bash
cd /Users/yangchen/Desktop/dora-manager
cargo test -p dm-core panel
cargo clippy -p dm-core -- -D warnings
```
