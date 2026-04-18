use axum::extract::{Path, State};
use axum::http::header::{self, HeaderValue};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::process::Command;

use crate::handlers::err;
use crate::state::AppState;

use utoipa::ToSchema;

/// GET /api/nodes
#[utoipa::path(get, path = "/api/nodes", responses((status = 200, description = "List of installed nodes")))]
pub async fn list_nodes(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::node::list_nodes(&state.home) {
        Ok(nodes) => Json(nodes).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/nodes/:id
#[utoipa::path(get, path = "/api/nodes/{id}", params(("id" = String, Path, description = "Node ID")), responses((status = 200, description = "Node details")))]
pub async fn node_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::node::node_status(&state.home, &id) {
        Ok(Some(entry)) => Json(entry).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, format!("Node '{}' not found", id)).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct InstallNodeRequest {
    pub id: String,
}

/// POST /api/nodes/install
#[utoipa::path(post, path = "/api/nodes/install", request_body = InstallNodeRequest, responses((status = 200, description = "Installed node")))]
pub async fn install_node(
    State(state): State<AppState>,
    Json(req): Json<InstallNodeRequest>,
) -> impl IntoResponse {
    match dm_core::node::install_node(&state.home, &req.id).await {
        Ok(entry) => Json(entry).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ImportNodeRequest {
    /// Local path or git URL
    pub source: String,
    /// Override node id (default: inferred from source basename)
    pub id: Option<String>,
}

/// POST /api/nodes/import
#[utoipa::path(post, path = "/api/nodes/import", request_body = ImportNodeRequest, responses((status = 200, description = "Imported node")))]
pub async fn import_node(
    State(state): State<AppState>,
    Json(req): Json<ImportNodeRequest>,
) -> impl IntoResponse {
    let is_url = req.source.starts_with("https://") || req.source.starts_with("http://");

    let inferred_id = req.id.unwrap_or_else(|| {
        if is_url {
            req.source
                .rsplit('/')
                .find(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string()
        } else {
            std::path::Path::new(&req.source)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        }
    });

    let result = if is_url {
        dm_core::node::import_git(&state.home, &inferred_id, &req.source).await
    } else {
        let source_path = std::path::Path::new(&req.source);
        let abs_path = if source_path.is_absolute() {
            source_path.to_path_buf()
        } else {
            // Relative paths resolve against dm home
            state.home.join(source_path)
        };
        dm_core::node::import_local(&state.home, &inferred_id, &abs_path)
    };

    match result {
        Ok(node) => Json(node).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UninstallNodeRequest {
    pub id: String,
}

/// POST /api/nodes/uninstall
#[utoipa::path(post, path = "/api/nodes/uninstall", request_body = UninstallNodeRequest, responses((status = 200, description = "Uninstall result")))]
pub async fn uninstall_node(
    State(state): State<AppState>,
    Json(req): Json<UninstallNodeRequest>,
) -> impl IntoResponse {
    match dm_core::node::uninstall_node(&state.home, &req.id) {
        Ok(()) => Json(serde_json::json!({ "message": format!("Uninstalled node '{}'", req.id) }))
            .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateNodeRequest {
    pub id: String,
    #[serde(default)]
    pub description: String,
}

/// POST /api/nodes/create
#[utoipa::path(post, path = "/api/nodes/create", request_body = CreateNodeRequest, responses((status = 200, description = "Created node")))]
pub async fn create_node(
    State(state): State<AppState>,
    Json(req): Json<CreateNodeRequest>,
) -> impl IntoResponse {
    match dm_core::node::create_node(&state.home, &req.id, &req.description) {
        Ok(entry) => Json(entry).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn node_readme(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Try to get local readme first
    if let Ok(content) = dm_core::node::get_node_readme(&state.home, &id) {
        return content.into_response();
    }

    (format!("No README found locally for '{}'", id),).into_response()
}

/// GET /api/nodes/:id/config
#[utoipa::path(get, path = "/api/nodes/{id}/config", params(("id" = String, Path, description = "Node ID")), responses((status = 200, description = "Node configuration")))]
pub async fn get_node_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::node::get_node_config(&state.home, &id) {
        Ok(config) => Json(config).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/nodes/:id/config
#[utoipa::path(post, path = "/api/nodes/{id}/config", params(("id" = String, Path, description = "Node ID")), responses((status = 200, description = "Config saved")))]
pub async fn save_node_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> impl IntoResponse {
    match dm_core::node::save_node_config(&state.home, &id, &config) {
        Ok(()) => Json(serde_json::json!({ "message": "Config saved" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// GET /api/nodes/:id/files
pub async fn get_node_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::node::git_like_file_tree(&state.home, &id) {
        Ok(files) => Json(files).into_response(),
        Err(e) => node_file_err(e, &id).into_response(),
    }
}

/// GET /api/nodes/:id/files/{*path}
pub async fn get_node_file_content(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(String, String)>,
) -> impl IntoResponse {
    match dm_core::node::read_node_file(&state.home, &id, &file_path) {
        Ok(content) => content.into_response(),
        Err(e) => node_file_err(e, &id).into_response(),
    }
}

/// GET /api/nodes/:id/artifacts/{*path}
pub async fn serve_node_artifact_file(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(String, String)>,
) -> impl IntoResponse {
    match dm_core::node::read_node_file_bytes(&state.home, &id, &file_path) {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
            let mut resp = bytes.into_response();
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            resp
        }
        Err(e) => node_file_err(e, &id).into_response(),
    }
}

fn node_file_err(e: anyhow::Error, id: &str) -> (StatusCode, String) {
    let message = e.to_string();
    if message.contains("Invalid node file path") {
        return (StatusCode::BAD_REQUEST, message);
    }
    if message.contains("does not exist")
        || message.contains("No such file or directory")
        || message == format!("Node '{}' not found", id)
    {
        return (StatusCode::NOT_FOUND, message);
    }

    (StatusCode::INTERNAL_SERVER_ERROR, message)
}

#[derive(Deserialize, ToSchema)]
pub struct OpenNodeRequest {
    pub target: String,
}

/// POST /api/nodes/:id/open
#[utoipa::path(post, path = "/api/nodes/{id}/open", params(("id" = String, Path, description = "Node ID")), request_body = OpenNodeRequest, responses((status = 200, description = "Opened node in external tool")))]
pub async fn open_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<OpenNodeRequest>,
) -> impl IntoResponse {
    let Some(node_path) = dm_core::node::resolve_node_dir(&state.home, &id) else {
        return (StatusCode::NOT_FOUND, format!("Node '{}' not found", id)).into_response();
    };

    let result = match req.target.as_str() {
        "finder" => Command::new("open").arg(&node_path).status(),
        "terminal" => Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&node_path)
            .status(),
        "vscode" => Command::new("open")
            .arg("-a")
            .arg("Visual Studio Code")
            .arg(&node_path)
            .status(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Unsupported open target '{}'", req.target),
            )
                .into_response();
        }
    };

    match result {
        Ok(status) if status.success() => Json(serde_json::json!({
            "message": format!("Opened '{}' in {}", id, req.target)
        }))
        .into_response(),
        Ok(status) => (
            StatusCode::BAD_REQUEST,
            format!("Failed to open '{}': launcher exited with {}", id, status),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("Failed to open '{}': {}", id, e),
        )
            .into_response(),
    }
}
