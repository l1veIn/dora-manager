The Runtime Service is the core engine of Dora Manager's backend, responsible for transforming a YAML dataflow topology into a managed **Run instance** and continuously tracking its state until termination. Spanning the `runs` module of `dm-core` and the HTTP/WebSocket layer of `dm-server`, it forms a complete "launch → monitor → collect → terminate" lifecycle management pipeline. Starting from an architectural overview, this article delves into the three major subsystems — startup orchestration, status refresh, and metrics collection — revealing the collaboration and design trade-offs between layers.

Sources: [mod.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/mod.rs#L1-L26), [service.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service.rs#L1-L45)

## Architecture Overview: Three-Layer Separation and Backend Abstraction

The Runtime Service's code organization follows the **separation of concerns** principle: the `model` layer defines pure data structures, the `repo` layer encapsulates filesystem I/O, and the `service` layer composes business logic. This layering is unified through `service.rs` as a facade file, providing external callers with a concise set of public functions.

Above all layers sits a key design abstraction — the **`RuntimeBackend` trait**. This trait encapsulates all interactions with the Dora CLI binary (start, stop, list) into a replaceable backend interface, enabling core business logic to be mocked in tests while using `DoraCliBackend` in production — an implementation that invokes the `dora` CLI via `tokio::process::Command`.

```
┌─────────────────────────────────────────────────────────┐
│                    dm-server (HTTP/WS)                    │
│  ┌──────────┐  ┌───────────┐  ┌──────────────────────┐  │
│  │ runs.rs  │  │ run_ws.rs │  │ runtime.rs (up/down) │  │
│  └────┬─────┘  └─────┬─────┘  └──────────┬───────────┘  │
│       │              │                    │               │
├───────┼──────────────┼────────────────────┼───────────────┤
│       ▼              ▼                    ▼               │
│  ┌─────────────────────────────────────────────────────┐ │
│  │              dm-core::runs (service layer)           │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ │ │
│  │  │service_start │ │service_runtime│ │service_metrics│ │ │
│  │  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘ │ │
│  │         │                │                 │         │ │
│  │  ┌──────┴────────────────┴─────────────────┴──────┐  │ │
│  │  │           RuntimeBackend trait                  │  │ │
│  │  │         (start_detached / stop / list)          │  │ │
│  │  └──────────────────┬─────────────────────────────┘  │ │
│  │                     │                                │ │
│  │  ┌──────────────────┴─────────────────────────────┐  │ │
│  │  │           DoraCliBackend                        │  │ │
│  │  │  dora start --detach / dora stop / dora list    │  │ │
│  │  └────────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

Sources: [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L14-L37), [service.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service.rs#L14-L44)

## RunInstance Data Model and Filesystem Layout

Each run instance corresponds to a `$DM_HOME/runs/<run_id>/` directory on disk, containing a complete state snapshot, transpilation artifacts, logs, and output files. `RunInstance` is the core persistence model, stored as `run.json` in JSON format.

**Filesystem Layout**:

| Path | Purpose |
|------|---------|
| `run.json` | Run instance metadata (state, timestamps, dora_uuid, etc.) |
| `dataflow.yml` | Original YAML snapshot |
| `view.json` | Optional canvas view state (editor position/zoom) |
| `dataflow.transpiled.yml` | Executable YAML processed through the transpilation pipeline |
| `logs/<node_id>.log` | Node logs synced from Dora output |
| `out/<dora_uuid>/log_<node>.txt` | Dora runtime's original output directory |

Sources: [repo.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/repo.rs#L1-L48)

**RunInstance Core Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `run_id` | `String` | UUID, globally unique identifier |
| `dora_uuid` | `Option<String>` | Dataflow UUID assigned by Dora runtime |
| `status` | `RunStatus` | `Running` / `Succeeded` / `Stopped` / `Failed` |
| `termination_reason` | `Option<TerminationReason>` | Termination reason enum (6 types) |
| `failure_node` / `failure_message` | `Option<String>` | Locates specific node and error summary on failure |
| `outcome` | `RunOutcome` | Human-readable summary containing `status`, `termination_reason`, `summary` |
| `node_count_expected` / `node_count_observed` | `u32` | Expected node count vs. actually observed node count |
| `log_sync` | `RunLogSync` | Log sync status (`Pending` / `Synced`) |
| `source` | `RunSource` | Launch source: `Cli` / `Server` / `Web` / `Unknown` |

Sources: [model.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/model.rs#L119-L174)

## Startup Orchestration: From YAML to Run Instance

Starting a Run is a multi-stage orchestration process. Whether via CLI's `dm run` or HTTP API's `POST /api/runs/start`, all paths converge at the `start_run_from_yaml_with_source_and_strategy` function. The following flowchart shows the complete orchestration path:

```
┌─────────────────┐
│  Receive YAML    │
└────────┬────────┘
         ▼
┌─────────────────────────┐
│ Phase 1: Executability   │
│ inspect_yaml() → can_run?│
│ ├─ invalid_yaml → bail  │
│ └─ missing_nodes → bail │
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 2: Conflict Check  │
│ Is same-name dataflow    │
│ already running?         │
│ ├─ Fail → error exit    │
│ └─ StopAndRestart → stop│
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 3: Prepare Env     │
│ ├─ Generate run_id (UUID)│
│ ├─ Create directory layout│
│ ├─ Save original YAML    │
│ ├─ Save view.json (opt)  │
│ └─ Compute dataflow_hash │
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 4: Dataflow        │
│ Transpilation            │
│ transpile_graph_for_run()│
│ → dataflow.transpiled.yml│
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 5: Build and       │
│ Persist RunInstance      │
│ → save_run(run.json)    │
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 6: Call Dora Start │
│ backend.start_detached() │
│ ├─ Ok(Some(uuid)) → ok  │
│ ├─ Ok(None) → mark fail │
│ └─ Err → mark fail      │
└─────────────────────────┘
```

Sources: [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L72-L220)

### Phase 1: Executability Check

Before starting, `inspect_yaml()` performs static analysis on the YAML. This function parses the YAML and checks each node's declared path, confirming whether the corresponding `dm.json` file exists in the `$DM_HOME/nodes/` directory. If there are missing nodes (`missing_nodes`) or invalid YAML (`invalid_yaml`), it returns an error immediately, avoiding submission of a doomed dataflow to the Dora runtime.

Sources: [inspect.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/dataflow/inspect.rs#L19-L39), [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L81-L103)

### Phase 2: Conflict Detection and Strategy Selection

The `StartConflictStrategy` enum defines two conflict handling strategies: **`Fail`** (default — immediately error when a same-name dataflow is already running) and **`StopAndRestart`** (stop the existing run first, then restart). Conflict detection is implemented by `find_active_run_by_name_with_backend()` — this function refreshes all run statuses first, then matches by `dataflow_name` among active Runs.

At the HTTP layer, `POST /api/runs/start` maps the `force` parameter to strategy selection. When `force` is not specified or `force=false`, the `Fail` strategy is used; when `force=true`, the `StopAndRestart` strategy is used.

Sources: [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L105-L118), [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L222-L228)

### Phase 3-5: Environment Preparation and Transpilation

A UUID is generated as `run_id`, the complete directory structure is created, and the original YAML is saved as a `dataflow.yml` snapshot. Then `transpile_graph_for_run()` executes a multi-pass transpilation pipeline (path resolution, port validation, config merging, runtime environment injection), producing `dataflow.transpiled.yml`. Finally, a `RunInstance` object is built — at this point `status` is `Running`, `dora_uuid` is `None`, and `outcome.summary` is "Running" — and persisted to disk.

Sources: [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L120-L174), [transpile/mod.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/dataflow/transpile/mod.rs#L36-L77)

### Phase 6: Dora Process Startup

`DoraCliBackend::start_detached()` executes `dora start <transpiled_path> --detach` via `tokio::process::Command`. This command starts the dataflow in detached mode; after the Dora CLI returns, the dataflow continues running in the background. The key signal for successful startup is a line in the output matching the format `dataflow start triggered: <uuid>` or `dataflow started: <uuid>` — the `extract_dataflow_id()` function extracts the `dora_uuid` from this.

If startup succeeds but no UUID is returned, the `RunInstance` is immediately marked as `Failed` (`termination_reason: StartFailed`). If the startup process itself throws an exception, it is also marked as `Failed` with error details recorded.

Sources: [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L39-L73), [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L176-L220)

### Additional Guarantees at the Server Layer

The `dm-server`'s `start_run` handler adds two safeguards before calling the core startup logic: **media backend readiness check** (if the dataflow contains media nodes but MediaMTX is not ready, startup is rejected) and **runtime auto-start** (calling `ensure_runtime_up()` to ensure the Dora daemon is running). `ensure_runtime_up()` probes the runtime state via `dora check`, and if not running, automatically executes `dora up`, waiting up to 5 seconds.

Sources: [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L195-L220), [api/runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/api/runtime.rs#L248-L256)

## Status Refresh: Synchronization Protocol with Dora Runtime

`refresh_run_statuses()` is the core function for status management, responsible for synchronizing locally recorded Run states with the actual state of the Dora runtime. This function is called before any query operation (`list_runs`, `get_run`, `list_active_runs`) to ensure callers see the most up-to-date state.

### Synchronization Algorithm

```
refresh_run_statuses_with_backend():
  1. Call backend.list() to get all active dataflows in Dora runtime
  2. Build runtime_map: {dora_uuid → RunStatus}
  3. Iterate all locally status.is_running() Runs:
     ├─ Found Running in runtime_map → keep Running
     ├─ Found Succeeded in runtime_map → mark complete + sync_run_outputs()
     ├─ Found Failed in runtime_map → infer failure details + sync_run_outputs()
     ├─ Found Stopped in runtime_map → mark RuntimeStopped + sync_run_outputs()
     └─ Not found in runtime_map → mark RuntimeLost + sync_run_outputs()
```

**`RuntimeLost`** is a notable state — when local records show a Run is running but the Dora runtime no longer reports that dataflow, it indicates an unexpected loss (e.g., Dora daemon restart). The system marks it as `Stopped` with a `RuntimeLost` reason, ensuring the state never gets stuck at `Running`.

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L99-L214)

### Log Synchronization: sync_run_outputs()

When a Run enters a terminal state, `sync_run_outputs()` copies the original logs from Dora runtime output (`out/<dora_uuid>/log_<node>.txt`) to the canonical location (`logs/<node_id>.log`), while updating the `nodes_observed` list and `log_sync` status. This sync step ensures that logs remain accessible in the `logs/` directory even after the Dora runtime has cleaned up the original output directory.

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L216-L263)

### Failure Inference: infer_failure_details()

When Dora reports a dataflow failure, it only provides coarse-grained status information. `infer_failure_details()` enriches failure details through a two-step strategy: first, it tries to parse `"node &lt;name&gt; failed:"` format information from the `dora stop` error message; if that is empty, it iterates through all observed node log files and extracts the first error summary using heuristic rules (`AssertionError:`, `thread 'main' panicked at`, `ERROR`, Python Traceback, etc.).

Sources: [state.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/state.rs#L34-L57), [state.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/state.rs#L116-L150)

### Terminal State Transition: apply_terminal_state()

`apply_terminal_state()` is the unified entry point for all terminal state transitions. It applies information from `TerminalStateUpdate` to `RunInstance` and calls `build_outcome()` to generate a human-readable summary text. This function guarantees that the `stopped_at` timestamp is set on first transition (idempotent) and updates `runtime_observed_at` to the observation time.

The summary generation logic of `build_outcome()` is as follows:

| Status | Condition | Summary Text |
|--------|-----------|-------------|
| `Running` | — | "Running" |
| `Succeeded` | — | "Succeeded" |
| `Stopped` | `StoppedByUser` | "Stopped by user" |
| `Stopped` | `RuntimeLost` | "Stopped after Dora runtime lost track of the dataflow" |
| `Stopped` | `RuntimeStopped` | "Stopped by Dora runtime" |
| `Failed` | node + message | "Failed: \<node\> \<message\>" |
| `Failed` | node only | "Failed: \<node\>" |
| `Failed` | message only | "Failed: \<message\>" |

Sources: [state.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/state.rs#L59-L114)

## Metrics Collection: CPU, Memory, and Node-Level Monitoring

The metrics collection system calls two CLI commands via `DoraCliBackend` — `dora list --format json` and `dora node list --format json --dataflow <uuid>` — to obtain real-time metrics at the dataflow level and node level respectively.

### Two-Level Metrics Structure

**Dataflow Level** (`RunMetrics`):

| Field | Type | Source |
|-------|------|--------|
| `cpu` | `Option<f64>` | `cpu` field from `dora list` JSON (percentage) |
| `memory_mb` | `Option<f64>` | `memory` field from `dora list` JSON (GB → MB conversion) |
| `nodes` | `Vec<NodeMetrics>` | Per-node details from `dora node list` |

**Node Level** (`NodeMetrics`):

| Field | Type | Example Source |
|-------|------|---------------|
| `id` | `String` | `"dora-qwen"` |
| `status` | `String` | `"Running"` |
| `pid` | `Option<String>` | `"67842"` |
| `cpu` | `Option<String>` | `"23.7%"` |
| `memory` | `Option<String>` | `"1143 MB"` |

Sources: [service_metrics.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_metrics.rs#L1-L96), [model.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/model.rs#L205-L219)

### Collection Modes

The system provides two collection functions:

- **`get_run_metrics(home, run_id)`**: Single Run metrics collection. First loads the Run to confirm it is in `Running` state with a `dora_uuid`, then sequentially calls dataflow-level and node-level metrics collection.
- **`collect_all_active_metrics(home)`**: Batch collection of metrics for all active dataflows. First obtains aggregated metrics for all dataflows in one call, then collects node-level metrics individually for each UUID. This method is used by the HTTP API `GET /api/runs/active?metrics=true`.

Both functions parse **newline-delimited JSON** (NDJSON) format — the Dora CLI outputs results as one JSON object per line. The parser is fault-tolerant: unparseable lines are silently skipped, and missing fields return `None`.

Sources: [service_metrics.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_metrics.rs#L14-L55), [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L64-L92)

## Stopping and Cleanup

### Stop Flow

`stop_run()` executes `dora stop <dora_uuid>` through `DoraCliBackend::stop()` with a 15-second timeout. The stop process includes a **fault tolerance mechanism**: if the `dora stop` command fails, the system calls `backend.list()` again to check whether the dataflow has actually stopped. If it has, it still marks it as `Stopped` (`StoppedByUser`), avoiding false `Failed` reports.

At the HTTP layer, `POST /api/runs/:id/stop` adopts a **fire-and-forget** pattern — the stop operation is `tokio::spawn`ed into a background task, and the HTTP response immediately returns `{"status": "stopping"}`, preventing the client from blocking while waiting for `dora stop` to timeout.

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L16-L97), [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L258-L284)

### Cleanup and Deletion

`delete_run()` deletes the Run directory and all its files, along with associated event records (`EventStore::delete_by_case_id()`). `clean_runs(home, keep)` retains the most recent `keep` records and deletes the remaining history.

Sources: [service_admin.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_admin.rs#L1-L28)

## Real-Time Push: WebSocket Endpoint

`dm-server` provides a WebSocket endpoint via `GET /api/runs/:id/ws` for real-time log streaming and metrics push during runtime. Once established, an event loop runs in the background, handling four types of events simultaneously via `tokio::select!`:

| Event Source | Interval | Push Content |
|-------------|----------|-------------|
| File change notification (`notify` crate) | Real-time | `WsMessage::Logs` (new log lines) and `WsMessage::Io` (interactive lines with `[DM-IO]` marker) |
| Metrics polling | 1 second | `WsMessage::Metrics` (node-level metrics) + `WsMessage::Status` (Run status) |
| Heartbeat | 10 seconds | `WsMessage::Ping` |
| Client messages | — | Detects `Close` frames to disconnect |

The file watcher uses `notify::recommended_watcher` to monitor the log directory. When the log directory switches from `out/<dora_uuid>/` (Dora real-time output) to `logs/` (archived after sync), the watcher automatically switches to the new directory. Each log read maintains a `log_offsets` hash table, pushing only incremental content to avoid duplicate transmission.

Sources: [run_ws.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/run_ws.rs#L1-L149), [run_ws.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/run_ws.rs#L207-L228)

## Background Idle Monitoring

When `dm-server` starts, it registers a background task that executes `auto_down_if_idle()` every 30 seconds. This function first calls `refresh_run_statuses()` to update all Run states (which detects naturally completed dataflows), then checks whether there are still active Runs. If not, it automatically executes `dora down` to shut down the Dora runtime and free system resources.

Sources: [main.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/main.rs#L234-L241), [api/runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/api/runtime.rs#L260-L270)

## RuntimeBackend Design Analysis

The `RuntimeBackend` trait defines three methods, covering the complete lifecycle of runtime interaction:

```rust
pub trait RuntimeBackend {
    fn start_detached<'a>(&'a self, home: &'a Path, transpiled_path: &'a Path) 
        -> BoxFutureResult<'a, (Option<String>, String)>;
    fn stop<'a>(&'a self, home: &'a Path, dora_uuid: &'a str) 
        -> BoxFutureResult<'a, ()>;
    fn list(&self, home: &Path) -> Result<Vec<RuntimeDataflow>>;
}
```

`DoraCliBackend` as the default implementation maps its methods to Dora CLI commands:

| Trait Method | CLI Command | Return Value |
|-------------|------------|--------------|
| `start_detached` | `dora start &lt;path&gt; --detach` | `(Option<uuid>, output_text)` |
| `stop` | `dora stop <uuid>` (15s timeout) | `()` |
| `list` | `dora list` | `Vec<RuntimeDataflow>` |

`start_detached` returns `BoxFutureResult` (i.e., `Pin<Box<dyn Future<Output = Result<T>> + Send>>`) because the startup operation needs to asynchronously wait for the subprocess to complete. `list` is synchronous (using `std::process::Command`) because the status refresh path needs to return quickly. All internal functions accepting `RuntimeBackend` use the generic constraint `B: RuntimeBackend`, enabling unit tests to inject mock backends.

Sources: [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L1-L37), [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L39-L119)

## HTTP API Route Overview

The following table summarizes all HTTP endpoints exposed by the Runtime Service:

| Method | Path | Core Function | Description |
|--------|------|--------------|-------------|
| GET | `/api/runs` | `list_runs_filtered` | Paginated query, supports status/search filtering |
| GET | `/api/runs/active` | `list_runs_filtered` + `collect_all_active_metrics` | Active Run list (optionally with metrics) |
| GET | `/api/runs/:id` | `get_run` + `get_run_metrics` | Run details (optionally with metrics) |
| GET | `/api/runs/:id/metrics` | `get_run_metrics` | Single Run metrics |
| POST | `/api/runs/start` | `start_run_from_yaml_with_source_and_strategy` | Start new Run |
| POST | `/api/runs/:id/stop` | `stop_run` | Stop Run (async background) |
| POST | `/api/runs/delete` | `delete_run` | Batch delete Runs |
| GET | `/api/runs/:id/dataflow` | `read_run_dataflow` | Original YAML snapshot |
| GET | `/api/runs/:id/transpiled` | `read_run_transpiled` | Transpiled YAML |
| GET | `/api/runs/:id/view` | `read_run_view` | Canvas view JSON |
| GET | `/api/runs/:id/logs/:node` | `read_run_log` | Full node log |
| GET | `/api/runs/:id/logs/:node/tail` | `read_run_log_chunk` | Incremental log (offset parameter) |
| GET | `/api/runs/:id/ws` | WebSocket handler | Real-time log + metrics push |
| POST | `/api/up` | `ensure_runtime_up` | Start Dora runtime |
| POST | `/api/down` | `down` | Stop Dora runtime |

Sources: [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L1-L333), [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runtime.rs#L69-L84), [main.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/main.rs#L174-L213)

## Design Highlights and Trade-offs

**Defensive State Management**: `refresh_run_statuses()` silently returns (`Err(_) => return Ok(())`) when `backend.list()` fails, rather than propagating the error. This is a deliberate design choice — when the Dora daemon is unreachable, the system does not misjudge all active Runs as failed, but retains the last known state, waiting for the next refresh opportunity.

**Detached Mode Startup**: Using the `--detach` flag to start dataflows means the `dm-server` process does not become the parent process of the dataflow. Even if the server restarts, started dataflows continue running under the Dora daemon's management — after the server restarts, `refresh_run_statuses()` rediscovers and takes over these "orphan" Runs.

**Fault-Tolerant Stop**: `dora stop` may return errors in certain edge cases (node already exited on its own, timeout, etc.) even though the dataflow has actually stopped. The secondary confirmation mechanism avoids misclassifying such scenarios as `Failed`.

**Metrics Collection Performance Considerations**: `collect_all_active_metrics()` calls `dora node list` for each active dataflow, meaning N active dataflows trigger N+1 CLI calls (1 `dora list` + N `dora node list`). The current design trades process invocation overhead for implementation simplicity — acceptable for the typical 1-3 active dataflow scenario.

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L108-L116), [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L176-L180), [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L50-L77)

---

**Further Reading**: For the complete lifecycle concepts and state machine of run instances, see [Run Instance: Lifecycle, State, and Metrics Tracking](06-run-lifecycle). For multi-pass processing details of the dataflow transpilation pipeline, see [Dataflow Transpiler: Multi-Pass Pipeline and Four-Layer Config Merging](08-transpiler). For the complete route definitions and Swagger documentation of the HTTP API, see [HTTP API Route Overview and Swagger Documentation](12-http-api). For how runtime events are recorded and exported, see [Event System: Observability Model and XES-Compatible Storage](11-event-system).
