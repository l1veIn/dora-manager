use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::state::AppState;

use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct LogTailParams {
    pub offset: Option<u64>,
}

#[derive(Deserialize, ToSchema)]
pub struct StartRunRequest {
    pub yaml: String,
    pub name: Option<String>,
    pub force: Option<bool>,
    pub view_json: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteRunsRequest {
    pub run_ids: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ActiveRunParams {
    pub metrics: Option<bool>,
}


/// GET /api/runs?limit=20&offset=0
#[utoipa::path(get, path = "/api/runs", params(("limit" = Option<i64>, Query), ("offset" = Option<i64>, Query), ("status" = Option<String>, Query), ("search" = Option<String>, Query)), responses((status = 200, description = "Paginated runs list")))]
pub async fn list_runs(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);
    let filter = dm_core::runs::RunListFilter {
        status: params.status,
        search: params.search,
    };

    match dm_core::runs::list_runs_filtered(&state.home, limit, offset, &filter) {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/runs/active
#[utoipa::path(get, path = "/api/runs/active", params(("metrics" = Option<bool>, Query)), responses((status = 200, description = "Active runs list")))]
pub async fn get_active_run(
    State(state): State<AppState>,
    Query(params): Query<ActiveRunParams>,
) -> impl IntoResponse {
    match dm_core::runs::list_runs_filtered(
        &state.home,
        10_000,
        0,
        &dm_core::runs::RunListFilter {
            status: Some("running".to_string()),
            search: None,
        },
    ) {
        Ok(mut result) => {
            if params.metrics.unwrap_or(false) {
                if let Ok(metrics_map) = dm_core::runs::collect_all_active_metrics(&state.home) {
                    for run in &mut result.runs {
                        if let Some(uuid) = run.dora_uuid.as_deref() {
                            run.metrics = metrics_map.get(uuid).cloned();
                        }
                    }
                }
            }
            Json(result.runs).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/runs/:id/metrics
#[utoipa::path(get, path = "/api/runs/{id}/metrics", params(("id" = String, Path)), responses((status = 200, description = "Run metrics")))]
pub async fn get_run_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::runs::get_run_metrics(&state.home, &id) {
        Ok(Some(metrics)) => Json(metrics).into_response(),
        Ok(None) => Json(dm_core::runs::RunMetrics::default()).into_response(),
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

/// GET /api/runs/:id/view
pub async fn get_run_view(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match dm_core::runs::read_run_view(&state.home, &id) {
        Ok(content) => content.into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct RunDetailParams {
    pub include_metrics: Option<bool>,
}

/// GET /api/runs/:id?include_metrics=true
#[utoipa::path(get, path = "/api/runs/{id}", params(("id" = String, Path), ("include_metrics" = Option<bool>, Query)), responses((status = 200, description = "Run detail")))]
pub async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<RunDetailParams>,
) -> impl IntoResponse {
    match dm_core::runs::get_run(&state.home, &id) {
        Ok(mut detail) => {
            if params.include_metrics.unwrap_or(false) {
                if let Ok(Some(m)) = dm_core::runs::get_run_metrics(&state.home, &id) {
                    detail.summary.metrics = Some(m);
                }
            }
            Json(detail).into_response()
        }
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
#[utoipa::path(post, path = "/api/runs/start", request_body = StartRunRequest, responses((status = 200, description = "Run started")))]
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
        req.view_json.as_deref(),
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
#[utoipa::path(post, path = "/api/runs/{id}/stop", params(("id" = String, Path)), responses((status = 200, description = "Run stop initiated")))]
pub async fn stop_run(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    // Verify the run exists before spawning, so we return 404 for invalid IDs.
    if dm_core::runs::load_run(&state.home, &id).is_err() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Run '{}' not found", id) })),
        )
            .into_response();
    }

    // Fire-and-forget: run the stop in background so the HTTP response returns immediately.
    let home = state.home.clone();
    let run_id = id.clone();
    tokio::spawn(async move {
        if let Err(e) = dm_core::runs::stop_run(&home, &run_id).await {
            eprintln!("[dm-server] Background stop_run({}) failed: {}", run_id, e);
        }
    });

    Json(serde_json::json!({
        "status": "stopping",
        "run_id": id,
    }))
    .into_response()
}

/// POST /api/runs/delete
#[utoipa::path(post, path = "/api/runs/delete", request_body = DeleteRunsRequest, responses((status = 200, description = "Deletion result")))]
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
