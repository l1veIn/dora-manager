use anyhow::Context;
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{graph::DataflowGraph, node};

use super::{error::ApiError, routes::AppState};

#[derive(Debug, Deserialize)]
pub struct InstallNodeRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct GraphRequest {
    pub yaml: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

fn invalid_json(err: JsonRejection) -> ApiError {
    ApiError::bad_request("Invalid request body", err.body_text())
}

fn internal_error(label: &str, err: anyhow::Error) -> ApiError {
    ApiError::internal(label, err.to_string())
}

pub async fn get_registry(State(state): State<AppState>) -> Result<Json<Vec<crate::registry::NodeMeta>>, ApiError> {
    let registry = state
        .load_registry()
        .await
        .context("Failed to load registry")
        .map_err(|e| internal_error("Failed to fetch registry", e))?;

    Ok(Json(registry))
}

pub async fn list_nodes(State(state): State<AppState>) -> Result<Json<Vec<crate::node::NodeEntry>>, ApiError> {
    let nodes = node::list_nodes(state.home.as_ref())
        .context("Failed to list installed nodes")
        .map_err(|e| internal_error("Failed to list nodes", e))?;

    Ok(Json(nodes))
}

pub async fn install_node(
    State(state): State<AppState>,
    payload: Result<Json<InstallNodeRequest>, JsonRejection>,
) -> Result<Json<crate::node::NodeEntry>, ApiError> {
    let Json(req) = payload.map_err(invalid_json)?;

    let registry = state
        .load_registry()
        .await
        .context("Failed to load registry")
        .map_err(|e| internal_error("Failed to install node", e))?;

    if crate::registry::find_node(&registry, &req.id).is_none() {
        return Err(ApiError::not_found(
            "Failed to install node",
            format!("Node '{}' not found in registry", req.id),
        ));
    }

    let entry = node::install_node(state.home.as_ref(), &req.id)
        .await
        .with_context(|| format!("Failed to install node '{}'", req.id))
        .map_err(|e| ApiError::bad_request("Failed to install node", e.to_string()))?;

    Ok(Json(entry))
}

pub async fn uninstall_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MessageResponse>, ApiError> {
    node::uninstall_node(state.home.as_ref(), &id)
        .with_context(|| format!("Failed to uninstall node '{}'", id))
        .map_err(|e| ApiError::bad_request("Failed to uninstall node", e.to_string()))?;

    Ok(Json(MessageResponse {
        message: format!("Uninstalled node '{}'", id),
    }))
}

pub async fn validate_graph(
    State(state): State<AppState>,
    payload: Result<Json<GraphRequest>, JsonRejection>,
) -> Result<Json<crate::graph::ValidationResult>, ApiError> {
    let Json(req) = payload.map_err(invalid_json)?;

    let graph = DataflowGraph::from_yaml_str(&req.yaml)
        .context("Failed to parse YAML graph")
        .map_err(|e| ApiError::bad_request("Failed to validate graph", e.to_string()))?;

    let registry = state
        .load_registry()
        .await
        .context("Failed to load registry")
        .map_err(|e| internal_error("Failed to validate graph", e))?;

    let result = crate::graph::validate_graph(&graph, &registry);
    Ok(Json(result))
}

pub async fn run_graph(
    payload: Result<Json<GraphRequest>, JsonRejection>,
) -> Result<Json<MessageResponse>, ApiError> {
    let _ = payload.map_err(invalid_json)?;

    Ok(Json(MessageResponse {
        message: "Not implemented yet".to_string(),
    }))
}
