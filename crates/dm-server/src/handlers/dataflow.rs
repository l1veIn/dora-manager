use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::handlers::{err, runs::StartRunRequest};
use crate::AppState;

/// GET /api/dataflows
pub async fn list_dataflows(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::dataflow::list(&state.home) {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/dataflows/:name
pub async fn get_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::get(&state.home, &name) {
        Ok(project) => Json(project).into_response(),
        Err(e) => {
            if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    return (
                        StatusCode::NOT_FOUND,
                        format!("Dataflow '{}' not found", name),
                    )
                        .into_response();
                }
            }
            err(e).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct SaveDataflowRequest {
    pub yaml: String,
}

#[derive(Deserialize)]
pub struct ImportDataflowsRequest {
    pub sources: Vec<String>,
}

/// POST /api/dataflows/:name
pub async fn save_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SaveDataflowRequest>,
) -> impl IntoResponse {
    match dm_core::dataflow::save(&state.home, &name, &req.yaml) {
        Ok(project) => Json(project).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/dataflows/import
pub async fn import_dataflows(
    State(state): State<AppState>,
    Json(req): Json<ImportDataflowsRequest>,
) -> impl IntoResponse {
    let normalized_sources: Vec<String> = req
        .sources
        .iter()
        .map(|source| {
            if source.starts_with("https://") || source.starts_with("http://") {
                return source.clone();
            }
            let path = std::path::Path::new(source);
            if path.is_absolute() {
                path.to_string_lossy().to_string()
            } else {
                state.home.join(path).to_string_lossy().to_string()
            }
        })
        .collect();
    let report = dm_core::dataflow::import_sources(&state.home, &normalized_sources).await;

    let status = if report.failed.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    (status, Json(report)).into_response()
}

/// POST /api/dataflows/:name/delete
pub async fn delete_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::delete(&state.home, &name) {
        Ok(()) => Json(serde_json::json!({ "message": "Deleted successfully" })).into_response(),
        Err(e) => {
            if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    return (
                        StatusCode::NOT_FOUND,
                        format!("Dataflow '{}' not found", name),
                    )
                        .into_response();
                }
            }
            err(e).into_response()
        }
    }
}

/// GET /api/dataflows/:name/meta
pub async fn get_dataflow_meta(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::get_flow_meta(&state.home, &name) {
        Ok(meta) => Json(meta).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// POST /api/dataflows/:name/meta
pub async fn save_dataflow_meta(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(meta): Json<dm_core::dataflow::FlowMeta>,
) -> impl IntoResponse {
    match dm_core::dataflow::save_flow_meta(&state.home, &name, &meta) {
        Ok(()) => Json(serde_json::json!({ "message": "Saved successfully" })).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/dataflows/:name/config
pub async fn get_dataflow_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::get_flow_config(&state.home, &name) {
        Ok(config) => Json(config).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// POST /api/dataflows/:name/config
pub async fn save_dataflow_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> impl IntoResponse {
    match dm_core::dataflow::save_flow_config(&state.home, &name, &config) {
        Ok(document) => Json(document).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/dataflows/:name/config-schema
pub async fn get_dataflow_config_schema(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::inspect_config(&state.home, &name) {
        Ok(document) => Json(document).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// GET /api/dataflows/:name/history
pub async fn list_dataflow_history(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::list_history(&state.home, &name) {
        Ok(history) => Json(history).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// GET /api/dataflows/:name/history/:version
pub async fn get_dataflow_history_version(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> impl IntoResponse {
    match dm_core::dataflow::get_history_version(&state.home, &name, &version) {
        Ok(yaml) => Json(serde_json::json!({ "yaml": yaml })).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// GET /api/dataflows/:name/inspect
pub async fn inspect_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::inspect(&state.home, &name) {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// POST /api/dataflows/:name/history/:version/restore
pub async fn restore_dataflow_history_version(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> impl IntoResponse {
    match dm_core::dataflow::restore_history_version(&state.home, &name, &version) {
        Ok(()) => Json(serde_json::json!({ "message": "Restored successfully" })).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

#[derive(Deserialize)]
pub struct RunDataflowRequest {
    pub yaml: String,
}

/// POST /api/dataflow/start
pub async fn start_dataflow(
    State(state): State<AppState>,
    Json(req): Json<RunDataflowRequest>,
) -> Response {
    crate::handlers::runs::start_run(
        State(state),
        Json(StartRunRequest {
            yaml: req.yaml,
            name: None,
            force: None,
        }),
    )
    .await
    .into_response()
}

/// POST /api/dataflow/stop
pub async fn stop_dataflow(State(state): State<AppState>) -> Response {
    match dm_core::runs::get_active_run(&state.home) {
        Ok(Some(run)) => crate::handlers::runs::stop_run(State(state), Path(run.run_id))
            .await
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No active run found." })),
        )
            .into_response(),
        Err(e) => err(e).into_response(),
    }
}

fn dataflow_not_found_or_err(e: anyhow::Error, name: &str) -> Response {
    if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
        if io_err.kind() == std::io::ErrorKind::NotFound {
            return (
                StatusCode::NOT_FOUND,
                format!("Dataflow '{}' not found", name),
            )
                .into_response();
        }
    }
    err(e).into_response()
}

/// GET /api/dataflows/:name/view
pub async fn get_dataflow_view(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::get_flow_view(&state.home, &name) {
        Ok(view) => Json(view).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// POST /api/dataflows/:name/view
pub async fn save_dataflow_view(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(view): Json<serde_json::Value>,
) -> impl IntoResponse {
    match dm_core::dataflow::save_flow_view(&state.home, &name, &view) {
        Ok(()) => Json(serde_json::json!({ "message": "View saved" })).into_response(),
        Err(e) => err(e).into_response(),
    }
}
