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
        Ok(content) => Json(serde_json::json!({ "yaml": content })).into_response(),
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

/// POST /api/dataflows/:name
pub async fn save_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SaveDataflowRequest>,
) -> impl IntoResponse {
    match dm_core::dataflow::save(&state.home, &name, &req.yaml) {
        Ok(()) => Json(serde_json::json!({ "message": "Saved successfully" })).into_response(),
        Err(e) => err(e).into_response(),
    }
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
