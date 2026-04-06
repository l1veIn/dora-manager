use axum::extract::State;
use axum::http::StatusCode;
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

/// GET /api/media/status
#[utoipa::path(get, path = "/api/media/status", responses((status = 200, description = "Media backend status")))]
pub async fn media_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.media.status().await).into_response()
}

/// POST /api/media/install
#[utoipa::path(post, path = "/api/media/install", responses((status = 200, description = "Media backend installed or resolved")))]
pub async fn install_media(State(state): State<AppState>) -> impl IntoResponse {
    match state.media.install().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
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
    pub media: Option<serde_json::Value>,
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

    if let Some(media) = req.media {
        match serde_json::from_value::<dm_core::config::MediaConfig>(media) {
            Ok(media) => cfg.media = media,
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("invalid media config: {}", err)
                    })),
                )
                    .into_response()
            }
        }
    }

    match dm_core::config::save_config(&state.home, &cfg) {
        Ok(()) => Json(cfg).into_response(),
        Err(e) => err(e).into_response(),
    }
}
