# feat/faas — FaaS Runtime

## Context

Build a standalone lightweight FaaS runtime (dm-faasd) that replaces the old service subsystem.
It's an HTTP server managing Python subprocess workers with warm pool + SSE streaming.

## Architecture

```
┌──────────────┐   HTTP POST/SSE   ┌──────────────────┐
│  dm-cli      │──────────────────▶│  dm-faasd (:5001) │
│  dm-core     │                   │                   │
│  curl/browser│                   │  WorkerPool per   │
└──────────────┘                   │  function         │
                                   │                   │
                                   │  Python subprocess│
                                   │  (STDIN/STDOUT    │
                                   │   JSON lines)     │
                                   └──────────────────┘
```

Functions live in `~/.dm/functions/<id>/service.json` + `service.py`.

## Files already created

- `crates/dm-faasd/Cargo.toml` — deps: axum 0.8, tokio, serde, anyhow, tracing, uuid
- `crates/dm-faasd/src/types.rs` — Function, InvokeRequest, FunctionEvent, HealthResponse
- `crates/dm-faasd/src/discovery.rs` — scan ~/.dm/functions/*/service.json
- `crates/dm-faasd/src/worker.rs` — WorkerPool, Worker (Python subprocess)
- Workspace Cargo.toml updated with dm-faasd member

## What Codex needs to do

### Phase 1: main.rs + routes (the glue)

Create `crates/dm-faasd/src/main.rs`:

```rust
mod discovery;
mod types;
mod worker;

use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Sse},
    routing::{get, post},
    Router,
};
use tokio::sync::Mutex;
use tracing::info;

use crate::discovery::{default_functions_dir, discover_functions};
use crate::types::*;
use crate::worker::Pools;

const IDLE_EXIT_SECS: u64 = 300;
const IDLE_CHECK_SECS: u64 = 30;
const DEFAULT_PORT: u16 = 5001;

struct AppState {
    start_time: Instant,
    functions: Mutex<Vec<Function>>,
    pools: worker::Pools,
    functions_dir: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let functions_dir = default_functions_dir();
    let functions = discover_functions(&functions_dir)?;
    info!("discovered {} functions", functions.len());

    let state = Arc::new(AppState {
        start_time: Instant::now(),
        functions: Mutex::new(functions),
        pools: Pools::new(IDLE_EXIT_SECS),
        functions_dir,
    });

    // Idle exit timer
    let idle_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(IDLE_CHECK_SECS));
        loop {
            interval.tick().await;
            if idle_state.start_time.elapsed().as_secs() > IDLE_EXIT_SECS {
                info!("dm-faasd idle timeout, exiting");
                std::process::exit(0);
            }
        }
    });

    // Worker reaper
    let reap_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            reap_state.pools.reap().await;
        }
    });

    // Rescan functions periodically
    let rescan_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok(fns) = discover_functions(&rescan_state.functions_dir) {
                *rescan_state.functions.lock().await = fns;
            }
        }
    });

    let app = Router::new()
        .route("/fn", get(list_functions))
        .route("/fn/{id}", get(get_function))
        .route("/fn/{id}/invoke", post(invoke_function))
        .route("/health", get(health_check))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    info!("dm-faasd listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn list_functions(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<serde_json::Value>> {
    let functions = state.functions.lock().await;
    let list: Vec<serde_json::Value> = functions
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "name": f.name,
                "version": f.version,
                "description": f.description,
                "methods": f.methods.iter().map(|m| serde_json::json!({
                    "name": m.name,
                    "description": m.description,
                    "events": m.events,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Json(list)
}

async fn get_function(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let functions = state.functions.lock().await;
    match functions.iter().find(|f| f.id == id) {
        Some(func) => Json(serde_json::json!({
            "id": func.id,
            "name": func.name,
            "version": func.version,
            "description": func.description,
            "entry": func.entry,
            "methods": func.methods,
            "runtime": {
                "max_workers": func.runtime.max_workers,
                "idle_timeout_secs": func.runtime.idle_timeout_secs,
            },
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "function not found"})),
        )
            .into_response(),
    }
}

async fn invoke_function(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<InvokeRequest>,
) -> impl IntoResponse {
    // Find function
    let func = {
        let functions = state.functions.lock().await;
        match functions.iter().find(|f| f.id == id) {
            Some(f) => f.clone(),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "function not found"})),
                )
                    .into_response();
            }
        }
    };

    // Verify method exists
    let has_method = func.methods.iter().any(|m| m.name == req.method);
    if !has_method {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("method '{}' not found", req.method),
                "code": "method_not_found",
            })),
        )
            .into_response();
    }

    // Get worker pool
    let pool = state.pools.for_function(&func).await;

    // Acquire worker
    let mut worker = match pool.acquire().await {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    // Build request JSON
    let request = serde_json::json!({
        "method": req.method,
        "input": req.input,
    });

    // Call
    match worker.call(&request).await {
        Ok(output) => Json(serde_json::json!({"output": output})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        uptime_secs: state.start_time.elapsed().as_secs(),
        functions: state.functions.lock().await.len(),
        workers: state.pools.total_workers().await,
    })
}
```

### Phase 2: demo function + test

Create `services/faas-demo/service.json`:
```json
{
  "id": "faas-demo",
  "name": "FaaS Demo",
  "version": "0.1.0",
  "description": "Minimal function for testing dm-faasd",
  "entry": "service.py",
  "methods": [
    {
      "name": "run",
      "description": "Echo input with an extra result field"
    }
  ],
  "runtime": {
    "max_workers": 2,
    "idle_timeout_secs": 300
  }
}
```

Create `services/faas-demo/service.py`:
```python
#!/usr/bin/env python3
import json, sys

for line in sys.stdin:
    req = json.loads(line)
    payload = req.get("input", {})
    payload["result"] = "ok"
    print(json.dumps(payload), flush=True)
```

### Phase 3: cargo build + verify

```bash
# Build
cargo build -p dm-faasd

# Set env so faasd can find the service.json
export DM_HOME=/tmp/test-dm
mkdir -p $DM_HOME/functions/faas-demo
cp services/faas-demo/service.json $DM_HOME/functions/faas-demo/
cp services/faas-demo/service.py $DM_HOME/functions/faas-demo/

# Run in background
cargo run -p dm-faasd &
sleep 2

# Test
curl http://127.0.0.1:5001/fn
# → should list faas-demo

curl -s http://127.0.0.1:5001/fn/faas-demo
# → should show metadata

curl -s http://127.0.0.1:5001/fn/faas-demo/invoke \
  -d '{"method":"run","input":{"hello":"world"}}'
# → should return {"output":{"hello":"world","result":"ok"}}

curl -s http://127.0.0.1:5001/health
# → should return OK

kill %1
```

### Phase 4: fix any compilation errors

Fix any issues until `cargo build` passes cleanly.

## Important notes

- The files `types.rs`, `discovery.rs`, `worker.rs` already exist with correct implementations
- Do NOT overwrite them — just create main.rs and the demo files
- The `Resolver = "2"` workspace setting is already in Cargo.toml
- Worker pool uses tokio processes, not std
- No external DB, no Redis, no cloud dependencies
