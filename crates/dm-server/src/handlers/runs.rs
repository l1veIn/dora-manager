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
}

/// GET /api/runs?limit=20&offset=0
pub async fn list_runs(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    match dm_core::runs::list_runs(&state.home, limit, offset) {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/runs/:id
pub async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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
    match dm_core::runs::get_run_logs(&state.home, &id, &node_id) {
        Ok(content) => content.into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

/// DELETE /api/runs/:id
pub async fn delete_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::runs::delete_run(&state.home, &id) {
        Ok(()) => Json(serde_json::json!({ "message": format!("Run '{}' deleted", id) }))
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
