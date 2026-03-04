mod handlers;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use rust_embed::Embed;
use tower_http::cors::CorsLayer;

use dm_core::events::EventStore;

#[derive(Embed)]
#[folder = "../../web/build"]
struct WebAssets;

#[derive(Clone)]
pub struct AppState {
    pub home: Arc<std::path::PathBuf>,
    pub events: Arc<EventStore>,
}

#[tokio::main]
async fn main() {
    let home = dm_core::config::resolve_home(None).expect("Failed to resolve dm home");

    let events = EventStore::open(&home).expect("Failed to open event store");

    let state = AppState {
        home: Arc::new(home),
        events: Arc::new(events),
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
        .route("/api/nodes", get(handlers::list_nodes))
        .route("/api/nodes/install", post(handlers::install_node))
        .route("/api/nodes/create", post(handlers::create_node))
        .route("/api/nodes/import", post(handlers::import_node))
        .route("/api/nodes/{id}", get(handlers::node_status))
        .route("/api/nodes/{id}/readme", get(handlers::node_readme))
        .route("/api/nodes/{id}/config", get(handlers::get_node_config))
        .route("/api/nodes/{id}/config", post(handlers::save_node_config))
        .route("/api/nodes/uninstall", post(handlers::uninstall_node))
        // ─── Dataflow Management ───
        .route("/api/dataflows", get(handlers::list_dataflows))
        .route("/api/dataflows/{name}", get(handlers::get_dataflow))
        .route("/api/dataflows/{name}", post(handlers::save_dataflow))
        .route(
            "/api/dataflows/{name}/delete",
            post(handlers::delete_dataflow),
        )
        // ─── Dataflow Execution ───
        .route("/api/dataflow/start", post(handlers::start_dataflow))
        .route("/api/dataflow/stop", post(handlers::stop_dataflow))
        // ─── Execution History (Runs) ───
        .route("/api/runs", get(handlers::list_runs))
        .route("/api/runs/{id}", get(handlers::get_run))
        .route("/api/runs/{id}", axum::routing::delete(handlers::delete_run))
        .route(
            "/api/runs/{id}/logs/{node_id}",
            get(handlers::get_run_logs),
        )
        // ─── Events / Observability ───
        .route("/api/events/count", get(handlers::count_events))
        .route("/api/events/export", get(handlers::export_events))
        .route("/api/events", get(handlers::query_events))
        .route("/api/events", post(handlers::ingest_event))
        // ─── Middleware ───
        .layer(CorsLayer::permissive())
        .with_state(state)
        // ─── Static Frontend Assets ───
        .fallback(axum::routing::get(handlers::serve_web));

    let addr = "127.0.0.1:3210";
    println!("🚀 dm-server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}
