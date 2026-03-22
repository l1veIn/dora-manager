# Phase 3: dm-server panel API

## Context

Read `docs/dm-panel-design.md` for architecture.
Phase 1 (dm-core) and Phase 2 (dm-cli) are already implemented.

## Task

Add panel REST endpoints to dm-server. All HTTP, zero WebSocket.

### 1. New handler file

Create `crates/dm-server/src/handlers/panel.rs`.

Follow the pattern in `handlers/runs.rs` — it's the closest analog:
same State extraction, same error handling, same JSON responses.

#### Endpoints

```rust
/// GET /api/panel/sessions
pub async fn list_sessions(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::panel::PanelStore::list_sessions(&state.home) {
        Ok(sessions) => Json(sessions).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/panel/:run_id/assets?since=0&input_id=camera&limit=100
#[derive(Deserialize)]
pub struct AssetQuery {
    since: Option<i64>,
    input_id: Option<String>,
    limit: Option<i64>,
}

pub async fn query_assets(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Query(params): Query<AssetQuery>,
) -> impl IntoResponse {
    match dm_core::panel::PanelStore::open(&state.home, &run_id) {
        Ok(store) => {
            let filter = AssetFilter {
                since_seq: params.since,
                input_id: params.input_id,
                limit: params.limit,
            };
            match store.query_assets(&filter) {
                Ok(result) => Json(result).into_response(),
                Err(e) => err(e).into_response(),
            }
        }
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

/// GET /api/panel/:run_id/file/*path
/// Serves binary asset files from ~/.dm/panel/<run_id>/<path>
pub async fn serve_asset_file(
    State(state): State<AppState>,
    Path((run_id, file_path)): Path<(String, String)>,
) -> impl IntoResponse {
    let full_path = state.home
        .join("panel")
        .join(&run_id)
        .join(&file_path);

    // Security: ensure resolved path is under panel/<run_id>/
    // Serve with appropriate Content-Type based on extension
    // Use tokio::fs::read for async file I/O
}

/// POST /api/panel/:run_id/commands
/// Body: { "output_id": "speed", "value": "0.5" }
#[derive(Deserialize)]
pub struct CommandBody {
    output_id: String,
    value: String,
}

pub async fn send_command(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Json(body): Json<CommandBody>,
) -> impl IntoResponse {
    match dm_core::panel::PanelStore::open(&state.home, &run_id) {
        Ok(store) => match store.write_command(&body.output_id, &body.value) {
            Ok(_) => Json(serde_json::json!({ "status": "ok" })).into_response(),
            Err(e) => err(e).into_response(),
        },
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}
```

### 2. Register routes

In `crates/dm-server/src/main.rs`, add:

```rust
// ─── Panel ───
.route("/api/panel/sessions", get(handlers::list_sessions_panel))
.route("/api/panel/{run_id}/assets", get(handlers::query_assets))
.route("/api/panel/{run_id}/file/*path", get(handlers::serve_asset_file))
.route("/api/panel/{run_id}/commands", post(handlers::send_command))
```

### 3. Register in handlers mod

In `crates/dm-server/src/handlers/mod.rs`, add:

```rust
mod panel;
pub use panel::*;
```

### 4. Performance note

For `query_assets` (polled every 50-100ms by browser), consider caching
the `PanelStore` connection in `AppState` rather than opening on every request.
Could use a `HashMap<String, PanelStore>` with entries for active sessions.
This is optional — opening SQLite is fast, optimize only if profiling shows need.

### Verification

```bash
cargo build -p dm-server
cargo test -p dm-server

# Manual test with curl:
# curl http://localhost:3210/api/panel/sessions
# curl http://localhost:3210/api/panel/<run_id>/assets?since=0
# curl -X POST http://localhost:3210/api/panel/<run_id>/commands \
#   -H 'Content-Type: application/json' \
#   -d '{"output_id":"speed","value":"0.5"}'
```
