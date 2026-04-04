use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::state::AppState;

use utoipa::ToSchema;

/// GET /api/doctor
#[utoipa::path(get, path = "/api/doctor", responses((status = 200, description = "System health report")))]
pub async fn doctor(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::doctor(&state.home).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/versions
#[utoipa::path(get, path = "/api/versions", responses((status = 200, description = "Installed dora versions")))]
pub async fn versions(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::versions(&state.home).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/status
#[utoipa::path(get, path = "/api/status", responses((status = 200, description = "Runtime and run status")))]
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::status(&state.home, false).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/config
#[utoipa::path(get, path = "/api/config", responses((status = 200, description = "DM configuration")))]
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::config::load_config(&state.home) {
        Ok(cfg) => Json(cfg).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ConfigUpdate {
    pub active_version: Option<String>,
}

/// POST /api/config
#[utoipa::path(post, path = "/api/config", request_body = ConfigUpdate, responses((status = 200, description = "Updated configuration")))]
pub async fn update_config(
    State(state): State<AppState>,
    Json(req): Json<ConfigUpdate>,
) -> impl IntoResponse {
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
