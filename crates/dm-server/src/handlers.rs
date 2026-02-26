use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::http::{header, Uri};
use serde::Deserialize;

use crate::{AppState, WebAssets};

// ─── Helper ───

fn err(e: impl std::fmt::Display) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

// ─── Environment Management ───

/// GET /api/doctor
pub async fn doctor(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::doctor(&state.home).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/versions
pub async fn versions(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::versions(&state.home).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/status
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::status(&state.home, false).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/config
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::config::load_config(&state.home) {
        Ok(cfg) => Json(cfg).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ConfigUpdate {
    pub active_version: Option<String>,
}

/// PUT /api/config
pub async fn update_config(
    State(state): State<AppState>,
    Json(req): Json<ConfigUpdate>,
) -> impl IntoResponse {
    // Load existing config and merge
    let mut cfg = match dm_core::config::load_config(&state.home) {
        Ok(c) => c,
        Err(e) => return err(e).into_response(),
    };

    if let Some(ver) = req.active_version {
        cfg.active_version = Some(ver);
    }

    match dm_core::config::save_config(&state.home, &cfg) {
        Ok(()) => Json(cfg).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct InstallRequest {
    pub version: Option<String>,
}

/// POST /api/install
pub async fn install(
    State(state): State<AppState>,
    Json(req): Json<InstallRequest>,
) -> impl IntoResponse {
    match dm_core::install::install(&state.home, req.version, false, None).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UninstallRequest {
    pub version: String,
}

/// POST /api/uninstall
pub async fn uninstall(
    State(state): State<AppState>,
    Json(req): Json<UninstallRequest>,
) -> impl IntoResponse {
    match dm_core::uninstall(&state.home, &req.version).await {
        Ok(()) => Json(serde_json::json!({ "message": format!("Uninstalled {}", req.version) }))
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UseRequest {
    pub version: String,
}

/// POST /api/use
pub async fn use_version(
    State(state): State<AppState>,
    Json(req): Json<UseRequest>,
) -> impl IntoResponse {
    match dm_core::use_version(&state.home, &req.version).await {
        Ok(actual_ver) => Json(serde_json::json!({
            "version": req.version,
            "actual_version": actual_ver,
        }))
        .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// POST /api/up
pub async fn up(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::up(&state.home, false).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/down
pub async fn down(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::down(&state.home, false).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

// ─── Node Management ───

/// GET /api/registry
pub async fn get_registry() -> impl IntoResponse {
    match dm_core::registry::fetch_registry().await {
        Ok(nodes) => Json(nodes).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/nodes
pub async fn list_nodes(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::node::list_nodes(&state.home) {
        Ok(nodes) => Json(nodes).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/nodes/:id
pub async fn node_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::node::node_status(&state.home, &id) {
        Ok(Some(entry)) => Json(entry).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, format!("Node '{}' not found", id)).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct InstallNodeRequest {
    pub id: String,
}

/// POST /api/nodes/install
pub async fn install_node(
    State(state): State<AppState>,
    Json(req): Json<InstallNodeRequest>,
) -> impl IntoResponse {
    match dm_core::node::install_node(&state.home, &req.id).await {
        Ok(entry) => Json(entry).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UninstallNodeRequest {
    pub id: String,
}

/// POST /api/nodes/uninstall
pub async fn uninstall_node(
    State(state): State<AppState>,
    Json(req): Json<UninstallNodeRequest>,
) -> impl IntoResponse {
    match dm_core::node::uninstall_node(&state.home, &req.id) {
        Ok(()) => {
            Json(serde_json::json!({ "message": format!("Uninstalled node '{}'", req.id) }))
                .into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

// ─── Dataflow Execution ───

#[derive(Deserialize)]
pub struct RunDataflowRequest {
    pub yaml: String,
}

/// POST /api/dataflow/run
pub async fn run_dataflow(
    State(state): State<AppState>,
    Json(req): Json<RunDataflowRequest>,
) -> impl IntoResponse {
    // 1. Write YAML to a temporary file
    let run_dir = dm_core::config::resolve_home(None)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("run");
    let _ = std::fs::create_dir_all(&run_dir);

    let tmp_path = run_dir.join("_web_dataflow.yml");
    if let Err(e) = std::fs::write(&tmp_path, &req.yaml) {
        return err(e).into_response();
    }

    // 2. Transpile the graph (sandbox path resolution)
    let transpiled = match dm_core::dataflow::transpile_graph(&state.home, &tmp_path) {
        Ok(v) => v,
        Err(e) => return err(e).into_response(),
    };

    // 3. Write transpiled YAML
    let transpiled_path = run_dir.join("_web_dataflow_transpiled.yml");
    let transpiled_str = match serde_yaml::to_string(&transpiled) {
        Ok(s) => s,
        Err(e) => return err(e).into_response(),
    };
    if let Err(e) = std::fs::write(&transpiled_path, &transpiled_str) {
        return err(e).into_response();
    }

    // 4. Find the active dora binary
    let dora_bin = match dm_core::dora::active_dora_bin(&state.home) {
        Ok(bin) => bin,
        Err(e) => return err(e).into_response(),
    };

    // 5. Run with --detach so it doesn't block the HTTP response
    let result = tokio::process::Command::new(&dora_bin)
        .args(["start", &transpiled_path.to_string_lossy(), "--detach"])
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Json(serde_json::json!({
                "status": "started",
                "message": stdout,
            }))
            .into_response()
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            (StatusCode::INTERNAL_SERVER_ERROR, stderr).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/dataflow/stop
pub async fn stop_dataflow(State(state): State<AppState>) -> impl IntoResponse {
    let dora_bin = match dm_core::dora::active_dora_bin(&state.home) {
        Ok(bin) => bin,
        Err(e) => return err(e).into_response(),
    };

    let result = tokio::process::Command::new(&dora_bin)
        .arg("stop")
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Json(serde_json::json!({
                "status": "stopped",
                "message": stdout,
            }))
            .into_response()
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            (StatusCode::INTERNAL_SERVER_ERROR, stderr).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

// ─── Events / Observability ───

/// GET /api/events?source=core&case_id=...&limit=100
pub async fn query_events(
    State(state): State<AppState>,
    axum::extract::Query(filter): axum::extract::Query<dm_core::events::EventFilter>,
) -> impl IntoResponse {
    match state.events.query(&filter) {
        Ok(events) => Json(events).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/events — frontend analytics / generic event ingest
pub async fn ingest_event(
    State(state): State<AppState>,
    Json(event): Json<dm_core::events::Event>,
) -> impl IntoResponse {
    match state.events.emit(&event) {
        Ok(id) => Json(serde_json::json!({ "id": id })).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/events/export?source=dataflow&format=xes
pub async fn export_events(
    State(state): State<AppState>,
    axum::extract::Query(filter): axum::extract::Query<dm_core::events::EventFilter>,
) -> impl IntoResponse {
    match state.events.export_xes(&filter) {
        Ok(xes) => (
            [(axum::http::header::CONTENT_TYPE, "application/xml")],
            xes,
        )
            .into_response(),
        Err(e) => err(e).into_response(),
    }
}

// ─── Static Frontend ───

pub async fn serve_web(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.is_empty() {
        path = "index.html".to_string();
    }

    match WebAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // SPA fallback: return index.html for not found routes
            if let Some(index) = WebAssets::get("index.html") {
                let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], index.data).into_response()
            } else {
                (StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        }
    }
}
