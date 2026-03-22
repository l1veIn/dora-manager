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
        .route("/api/nodes/{id}/files", get(handlers::get_node_files))
        .route(
            "/api/nodes/{id}/files/{*path}",
            get(handlers::get_node_file_content),
        )
        .route("/api/nodes/{id}/config", get(handlers::get_node_config))
        .route("/api/nodes/{id}/config", post(handlers::save_node_config))
        .route("/api/nodes/uninstall", post(handlers::uninstall_node))
        // ─── Dataflow Management ───
        .route("/api/dataflows", get(handlers::list_dataflows))
        .route("/api/dataflows/import", post(handlers::import_dataflows))
        .route("/api/dataflows/{name}", get(handlers::get_dataflow))
        .route("/api/dataflows/{name}", post(handlers::save_dataflow))
        .route(
            "/api/dataflows/{name}/inspect",
            get(handlers::inspect_dataflow),
        )
        .route(
            "/api/dataflows/{name}/meta",
            get(handlers::get_dataflow_meta),
        )
        .route(
            "/api/dataflows/{name}/meta",
            post(handlers::save_dataflow_meta),
        )
        .route(
            "/api/dataflows/{name}/config",
            get(handlers::get_dataflow_config),
        )
        .route(
            "/api/dataflows/{name}/config-schema",
            get(handlers::get_dataflow_config_schema),
        )
        .route(
            "/api/dataflows/{name}/config",
            post(handlers::save_dataflow_config),
        )
        .route(
            "/api/dataflows/{name}/history",
            get(handlers::list_dataflow_history),
        )
        .route(
            "/api/dataflows/{name}/history/{version}",
            get(handlers::get_dataflow_history_version),
        )
        .route(
            "/api/dataflows/{name}/history/{version}/restore",
            post(handlers::restore_dataflow_history_version),
        )
        .route(
            "/api/dataflows/{name}/delete",
            post(handlers::delete_dataflow),
        )
        .route(
            "/api/dataflows/{name}/view",
            get(handlers::get_dataflow_view),
        )
        .route(
            "/api/dataflows/{name}/view",
            post(handlers::save_dataflow_view),
        )
        // ─── Dataflow Execution ───
        .route("/api/dataflow/start", post(handlers::start_dataflow))
        .route("/api/dataflow/stop", post(handlers::stop_dataflow))
        // ─── Execution History (Runs) ───
        .route("/api/runs", get(handlers::list_runs))
        .route("/api/runs/start", post(handlers::start_run))
        .route("/api/runs/active", get(handlers::get_active_run))
        .route("/api/runs/{id}", get(handlers::get_run))
        .route("/api/runs/{id}/metrics", get(handlers::get_run_metrics))
        .route("/api/runs/{id}/stop", post(handlers::stop_run))
        .route("/api/runs/{id}/dataflow", get(handlers::get_run_dataflow))
        .route(
            "/api/runs/{id}/transpiled",
            get(handlers::get_run_transpiled),
        )
        .route("/api/runs/delete", post(handlers::delete_runs))
        .route("/api/runs/{id}/logs/{node_id}", get(handlers::get_run_logs))
        .route(
            "/api/runs/{id}/logs/{node_id}/tail",
            get(handlers::tail_run_logs),
        )
        // ─── Events / Observability ───
        .route("/api/events/count", get(handlers::count_events))
        .route("/api/events/export", get(handlers::export_events))
        .route("/api/events", get(handlers::query_events))
        .route("/api/events", post(handlers::ingest_event))
        // ─── Panel ───
        .route(
            "/api/runs/{run_id}/panel/assets",
            get(handlers::query_assets),
        )
        .route(
            "/api/runs/{run_id}/panel/file/{*path}",
            get(handlers::serve_asset_file),
        )
        .route(
            "/api/runs/{run_id}/panel/commands",
            post(handlers::send_command),
        )
        .route(
            "/api/runs/{run_id}/panel/command",
            post(handlers::send_command),
        )
        .route(
            "/api/runs/{run_id}/panel/widgets",
            get(handlers::get_widgets),
        )
        .route(
            "/api/runs/{run_id}/panel/ws",
            get(handlers::panel_ws),
        )
        .route(
            "/api/runs/{run_id}/panel/options/{input_id}",
            get(handlers::get_latest_option),
        )
        // ─── Middleware ───
        .layer(CorsLayer::permissive())
        .with_state(state.clone())
        // ─── Static Frontend Assets ───
        .fallback(axum::routing::get(handlers::serve_web));

    let addr = "127.0.0.1:3210";
    println!("🚀 dm-server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    // Background idle monitor: auto-down dora when no active runs remain
    let monitor_home = state.home.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            dm_core::auto_down_if_idle(&monitor_home, false).await;
        }
    });

    axum::serve(listener, app).await.expect("Server error");
}
