mod handlers;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub home: Arc<std::path::PathBuf>,
}

#[tokio::main]
async fn main() {
    let home = dm_core::config::resolve_home(None).expect("Failed to resolve dm home");

    let state = AppState {
        home: Arc::new(home),
    };

    let app = Router::new()
        // ─── Environment Management ───
        .route("/api/doctor", get(handlers::doctor))
        .route("/api/versions", get(handlers::versions))
        .route("/api/status", get(handlers::status))
        .route("/api/config", get(handlers::get_config))
        .route("/api/config", post(handlers::update_config))
        .route("/api/install", post(handlers::install))
        .route("/api/uninstall", post(handlers::uninstall))
        .route("/api/use", post(handlers::use_version))
        .route("/api/up", post(handlers::up))
        .route("/api/down", post(handlers::down))
        // ─── Node Management ───
        .route("/api/registry", get(handlers::get_registry))
        .route("/api/nodes", get(handlers::list_nodes))
        .route("/api/nodes/install", post(handlers::install_node))
        .route("/api/nodes/{id}", get(handlers::node_status))
        .route("/api/nodes/uninstall", post(handlers::uninstall_node))
        // ─── Dataflow Execution ───
        .route("/api/dataflow/run", post(handlers::run_dataflow))
        .route("/api/dataflow/stop", post(handlers::stop_dataflow))
        // ─── Middleware ───
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:3210";
    println!("🚀 dm-server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}
