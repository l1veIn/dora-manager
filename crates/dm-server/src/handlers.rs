use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::AppState;

/// GET /api/doctor
pub async fn doctor(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::doctor(&state.home).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/versions
pub async fn versions(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::versions(&state.home).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/status
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::status(&state.home, false).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// DELETE /api/uninstall/:version
pub async fn uninstall(
    State(state): State<AppState>,
    Path(version): Path<String>,
) -> impl IntoResponse {
    match dm_core::uninstall(&state.home, &version).await {
        Ok(()) => (StatusCode::OK, format!("Uninstalled {}", version)).into_response(),
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
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// POST /api/down
pub async fn down(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::down(&state.home, false).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
