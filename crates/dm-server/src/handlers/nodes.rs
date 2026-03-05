use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::AppState;

/// GET /api/nodes
pub async fn list_nodes(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::node::list_nodes(&state.home) {
        Ok(nodes) => Json(nodes).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/nodes/:id
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

#[derive(Deserialize)]
pub struct InstallNodeRequest {
    pub id: String,
}

/// POST /api/nodes/install
pub async fn install_node(
    State(state): State<AppState>,
    Json(req): Json<InstallNodeRequest>,
) -> impl IntoResponse {
    match dm_core::node::install_node(&state.home, &req.id).await {
        Ok(entry) => Json(entry).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ImportNodeRequest {
    /// Local path or git URL
    pub source: String,
    /// Override node id (default: inferred from source basename)
    pub id: Option<String>,
}

/// POST /api/nodes/import
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
        dm_core::node::import_local(&state.home, &inferred_id, &abs_path).map_err(|e| e.into())
    };

    match result {
        Ok(node) => Json(node).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UninstallNodeRequest {
    pub id: String,
}

/// POST /api/nodes/uninstall
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

#[derive(Deserialize)]
pub struct CreateNodeRequest {
    pub id: String,
    #[serde(default)]
    pub description: String,
}

/// POST /api/nodes/create
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
