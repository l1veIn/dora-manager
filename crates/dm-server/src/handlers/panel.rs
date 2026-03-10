use std::path::{Component, PathBuf};

use axum::extract::{Path, Query, State};
use axum::http::{header::CONTENT_TYPE, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::AppState;

struct PanelGuardError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for PanelGuardError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

#[derive(Deserialize)]
pub struct AssetQuery {
    pub since: Option<i64>,
    pub after: Option<i64>,
    pub cursor: Option<i64>,
    pub before: Option<i64>,
    pub input_id: Option<String>,
    pub limit: Option<i64>,
}

pub async fn query_assets(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Query(params): Query<AssetQuery>,
) -> Response {
    if let Err(response) = ensure_run_has_panel(&state.home, &run_id) {
        return response.into_response();
    }

    match dm_core::runs::panel::PanelStore::open(&state.home, &run_id) {
        Ok(store) => {
            let filter = dm_core::runs::panel::AssetFilter {
                since_seq: params.since.or(params.after).or(params.cursor),
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

    let full_path = dm_core::runs::run_panel_dir(&state.home, &run_id).join(relative);

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
    pub output_id: Option<String>,
    pub value: Option<String>,
    pub command: Option<String>,
}

pub async fn send_command(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Json(body): Json<CommandBody>,
) -> Response {
    if let Err(response) = ensure_run_has_panel(&state.home, &run_id) {
        return response.into_response();
    }

    match dm_core::runs::panel::PanelStore::open(&state.home, &run_id) {
        Ok(store) => {
            let output_id = body.output_id.as_deref().unwrap_or("input");
            let value = body.value.as_deref().or(body.command.as_deref());
            let Some(value) = value else {
                return (
                    StatusCode::BAD_REQUEST,
                    "Missing command payload: provide `value` or `command`",
                )
                    .into_response();
            };
            match store.write_command(output_id, value) {
                Ok(_) => {
                    // Persist the sent value as the new default in widgets.json
                    if body.output_id.is_some() {
                        let widgets_path =
                            dm_core::runs::run_panel_dir(&state.home, &run_id)
                                .join("widgets.json");
                        if let Ok(content) = std::fs::read_to_string(&widgets_path) {
                            if let Ok(mut json) =
                                serde_json::from_str::<serde_json::Value>(&content)
                            {
                                if let Some(widget) = json.get_mut(output_id) {
                                    widget["default"] =
                                        serde_json::Value::String(value.to_string());
                                    if let Ok(updated) = serde_json::to_string_pretty(&json) {
                                        let _ = std::fs::write(&widgets_path, updated);
                                    }
                                }
                            }
                        }
                    }
                    Json(serde_json::json!({ "status": "ok" })).into_response()
                }
                Err(e) => err(e).into_response(),
            }
        }
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn get_widgets(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Response {
    let widgets_path = dm_core::runs::run_panel_dir(&state.home, &run_id).join("widgets.json");
    match tokio::fs::read_to_string(&widgets_path).await {
        Ok(content) => {
            let json: serde_json::Value =
                serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
            Json(json).into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Json(serde_json::json!({})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

fn ensure_run_has_panel(home: &std::path::Path, run_id: &str) -> Result<(), PanelGuardError> {
    let run = dm_core::runs::load_run(home, run_id).map_err(|e| PanelGuardError {
        status: StatusCode::NOT_FOUND,
        message: format!("Run '{}' not found: {}", run_id, e),
    })?;

    if !run.has_panel {
        return Err(PanelGuardError {
            status: StatusCode::BAD_REQUEST,
            message: format!("Run '{}' does not have a panel", run_id),
        });
    }

    Ok(())
}
