mod handlers;

use std::sync::Arc;

use axum::routing::{delete, get, post};
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
        .route("/api/doctor", get(handlers::doctor))
        .route("/api/versions", get(handlers::versions))
        .route("/api/status", get(handlers::status))
        .route("/api/install", post(handlers::install))
        .route("/api/uninstall/{version}", delete(handlers::uninstall))
        .route("/api/use", post(handlers::use_version))
        .route("/api/up", post(handlers::up))
        .route("/api/down", post(handlers::down))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:3210";
    println!("🚀 dm-server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}
