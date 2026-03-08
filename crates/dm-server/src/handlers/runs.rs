use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::AppState;

#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub has_panel: Option<bool>,
}

#[derive(Deserialize)]
pub struct LogTailParams {
    pub offset: Option<u64>,
}

#[derive(Deserialize)]
pub struct StartRunRequest {
    pub yaml: String,
    pub name: Option<String>,
    pub force: Option<bool>,
}

#[derive(Deserialize)]
pub struct DeleteRunsRequest {
    pub run_ids: Vec<String>,
}

/// GET /api/runs?limit=20&offset=0
pub async fn list_runs(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);
    let filter = dm_core::runs::RunListFilter {
        status: params.status,
        search: params.search,
        has_panel: params.has_panel,
    };

    match dm_core::runs::list_runs_filtered(&state.home, limit, offset, &filter) {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/runs/active
pub async fn get_active_run(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::runs::list_runs_filtered(
        &state.home,
        10_000,
        0,
        &dm_core::runs::RunListFilter {
            status: Some("running".to_string()),
            search: None,
            has_panel: None,
        },
    ) {
        Ok(result) => Json(result.runs).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/runs/:id/dataflow
pub async fn get_run_dataflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::runs::read_run_dataflow(&state.home, &id) {
        Ok(content) => content.into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

/// GET /api/runs/:id/transpiled
pub async fn get_run_transpiled(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::runs::read_run_transpiled(&state.home, &id) {
        Ok(content) => content.into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

/// GET /api/runs/:id
pub async fn get_run(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match dm_core::runs::get_run(&state.home, &id) {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

/// GET /api/runs/:id/logs/:node_id
pub async fn get_run_logs(
    State(state): State<AppState>,
    Path((id, node_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match dm_core::runs::read_run_log(&state.home, &id, &node_id) {
        Ok(content) => content.into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

/// GET /api/runs/:id/logs/:node_id/tail?offset=0
pub async fn tail_run_logs(
    State(state): State<AppState>,
    Path((id, node_id)): Path<(String, String)>,
    Query(params): Query<LogTailParams>,
) -> impl IntoResponse {
    match dm_core::runs::read_run_log_chunk(&state.home, &id, &node_id, params.offset.unwrap_or(0))
    {
        Ok(chunk) => Json(chunk).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

/// POST /api/runs/start
pub async fn start_run(
    State(state): State<AppState>,
    Json(req): Json<StartRunRequest>,
) -> impl IntoResponse {
    if let Err(e) = dm_core::ensure_runtime_up(&state.home, false).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to auto-start dora runtime: {}", e)
            })),
        )
            .into_response();
    }

    let dataflow_name = req.name.unwrap_or_else(|| "web-dataflow".to_string());

    let strategy = if req.force.unwrap_or(false) {
        dm_core::runs::StartConflictStrategy::StopAndRestart
    } else {
        dm_core::runs::StartConflictStrategy::Fail
    };

    match dm_core::runs::start_run_from_yaml_with_source_and_strategy(
        &state.home,
        &req.yaml,
        &dataflow_name,
        dm_core::runs::RunSource::Server,
        strategy,
    )
    .await
    {
        Ok(result) => Json(serde_json::json!({
            "status": "started",
            "message": result.message,
            "run_id": result.run.run_id,
            "run": result.run,
        }))
        .into_response(),
        Err(e) => {
            let text = e.to_string();
            if text.contains("already running as run") {
                (StatusCode::CONFLICT, text).into_response()
            } else {
                err(e).into_response()
            }
        }
    }
}

/// POST /api/runs/:id/stop
pub async fn stop_run(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match dm_core::runs::stop_run(&state.home, &id).await {
        Ok(run) => Json(serde_json::json!({
            "status": "stopped",
            "run": run,
        }))
        .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// POST /api/runs/delete
pub async fn delete_runs(
    State(state): State<AppState>,
    Json(req): Json<DeleteRunsRequest>,
) -> impl IntoResponse {
    if req.run_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "run_ids must not be empty"
            })),
        )
            .into_response();
    }

    let total = req.run_ids.len();
    let mut deleted = Vec::new();
    let mut failed = Vec::new();

    for run_id in req.run_ids {
        match dm_core::runs::delete_run(&state.home, &run_id) {
            Ok(()) => deleted.push(run_id),
            Err(e) => failed.push(serde_json::json!({
                "run_id": run_id,
                "error": e.to_string(),
            })),
        }
    }

    let status = if failed.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::MULTI_STATUS
    };

    (
        status,
        Json(serde_json::json!({
            "deleted": deleted,
            "failed": failed,
            "deleted_count": total - failed.len(),
            "failed_count": failed.len(),
        })),
    )
        .into_response()
}
