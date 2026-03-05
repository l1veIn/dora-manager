use std::path::{Component, PathBuf};

use axum::extract::{Path, Query, State};
use axum::http::{header::CONTENT_TYPE, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::AppState;

pub async fn list_sessions_panel(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::panel::PanelStore::list_sessions(&state.home) {
        Ok(sessions) => Json(sessions).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct AssetQuery {
    pub since: Option<i64>,
    pub before: Option<i64>,
    pub input_id: Option<String>,
    pub limit: Option<i64>,
}

pub async fn query_assets(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Query(params): Query<AssetQuery>,
) -> impl IntoResponse {
    if !panel_run_exists(&state.home, &run_id) {
        return (
            StatusCode::NOT_FOUND,
            format!("Panel run '{}' not found", run_id),
        )
            .into_response();
    }

    match dm_core::panel::PanelStore::open(&state.home, &run_id) {
        Ok(store) => {
            let filter = dm_core::panel::AssetFilter {
                since_seq: params.since,
                before_seq: params.before,
                input_id: params.input_id,
                limit: params.limit,
            };
            match store.query_assets(&filter) {
                Ok(result) => Json(result).into_response(),
                Err(e) => err(e).into_response(),
            }
        }
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn serve_asset_file(
    State(state): State<AppState>,
    Path((run_id, file_path)): Path<(String, String)>,
) -> impl IntoResponse {
    let relative = PathBuf::from(file_path);
    if relative.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return (StatusCode::BAD_REQUEST, "Invalid file path").into_response();
    }

    let full_path = state.home.join("panel").join(&run_id).join(relative);

    match tokio::fs::read(&full_path).await {
        Ok(bytes) => {
            let content_type = mime_guess::from_path(&full_path)
                .first_or_octet_stream()
                .to_string();
            ([(CONTENT_TYPE, content_type)], bytes).into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CommandBody {
    pub output_id: String,
    pub value: String,
}

pub async fn send_command(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Json(body): Json<CommandBody>,
) -> impl IntoResponse {
    if !panel_run_exists(&state.home, &run_id) {
        return (
            StatusCode::NOT_FOUND,
            format!("Panel run '{}' not found", run_id),
        )
            .into_response();
    }

    match dm_core::panel::PanelStore::open(&state.home, &run_id) {
        Ok(store) => match store.write_command(&body.output_id, &body.value) {
            Ok(_) => Json(serde_json::json!({ "status": "ok" })).into_response(),
            Err(e) => err(e).into_response(),
        },
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

fn panel_run_exists(home: &std::path::Path, run_id: &str) -> bool {
    home.join("panel").join(run_id).join("index.db").exists()
}
