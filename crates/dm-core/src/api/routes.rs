use std::{path::PathBuf, sync::Arc};

use axum::{routing::{delete, get, post}, Router};

use crate::registry::NodeMeta;

use super::handlers;

#[derive(Clone)]
pub struct AppState {
    pub home: Arc<PathBuf>,
    registry_override: Option<Arc<Vec<NodeMeta>>>,
}

impl AppState {
    pub fn new(home: Arc<PathBuf>) -> Self {
        Self {
            home,
            registry_override: None,
        }
    }

    pub fn with_registry_override(mut self, nodes: Vec<NodeMeta>) -> Self {
        self.registry_override = Some(Arc::new(nodes));
        self
    }

    pub(crate) async fn load_registry(&self) -> anyhow::Result<Vec<NodeMeta>> {
        if let Some(nodes) = &self.registry_override {
            return Ok((**nodes).clone());
        }

        crate::registry::fetch_registry().await
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/registry", get(handlers::get_registry))
        .route("/api/v1/nodes", get(handlers::list_nodes))
        .route("/api/v1/nodes/install", post(handlers::install_node))
        .route("/api/v1/nodes/{id}", delete(handlers::uninstall_node))
        .route("/api/v1/graph/run", post(handlers::run_graph))
        .with_state(state)
}
