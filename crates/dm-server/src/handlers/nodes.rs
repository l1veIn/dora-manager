use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::AppState;

/// GET /api/registry
pub async fn get_registry() -> impl IntoResponse {
    match dm_core::registry::fetch_registry().await {
        Ok(nodes) => Json(nodes).into_response(),
        Err(e) => err(e).into_response(),
    }
}

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

/// POST /api/nodes/download
pub async fn download_node(
    State(state): State<AppState>,
    Json(req): Json<InstallNodeRequest>,
) -> impl IntoResponse {
    match dm_core::node::download_node(&state.home, &req.id).await {
        Ok(entry) => Json(entry).into_response(),
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
        Ok(()) => {
            Json(serde_json::json!({ "message": format!("Uninstalled node '{}'", req.id) }))
                .into_response()
        }
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

    // Fallback: try to fetch from registry and online
    if let Ok(registry) = dm_core::registry::fetch_registry().await {
        if let Some(node) = dm_core::registry::find_node(&registry, &id) {
            if let Some(github_url) = &node.github {
                if github_url.starts_with("https://github.com/") {
                    let raw_url = github_url
                        .replace("https://github.com/", "https://raw.githubusercontent.com/")
                        .replace("/tree/", "/")
                        .replace("/blob/", "/");
                    let readme_url = format!("{}/README.md", raw_url.trim_end_matches('/'));
                    
                    if let Ok(resp) = reqwest::get(&readme_url).await {
                        if resp.status().is_success() {
                            if let Ok(text) = resp.text().await {
                                return text.into_response();
                            }
                        }
                    }
                }
            }
        }
    }

    (StatusCode::NOT_FOUND, format!("No README found locally or online for '{}'", id)).into_response()
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
