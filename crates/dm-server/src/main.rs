mod handlers;
pub mod services;
pub mod state;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use rust_embed::Embed;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use dm_core::events::EventStore;
pub use state::{AppState, InputEventNotification, InteractionNotification};

#[derive(Embed)]
#[folder = "../../web/build"]
struct WebAssets;

#[derive(OpenApi)]
#[openapi(
    paths(
        // System
        handlers::system::doctor,
        handlers::system::versions,
        handlers::system::status,
        handlers::system::get_config,
        handlers::system::update_config,
        // Runtime
        handlers::runtime::install,
        handlers::runtime::uninstall,
        handlers::runtime::use_version,
        handlers::runtime::up,
        handlers::runtime::down,
        // Nodes
        handlers::nodes::list_nodes,
        handlers::nodes::node_status,
        handlers::nodes::install_node,
        handlers::nodes::import_node,
        handlers::nodes::uninstall_node,
        handlers::nodes::create_node,
        handlers::nodes::get_node_config,
        handlers::nodes::save_node_config,
        // Dataflows
        handlers::dataflow::list_dataflows,
        handlers::dataflow::get_dataflow,
        handlers::dataflow::save_dataflow,
        handlers::dataflow::import_dataflows,
        handlers::dataflow::delete_dataflow,
        handlers::dataflow::start_dataflow,
        handlers::dataflow::stop_dataflow,
        // Runs
        handlers::runs::list_runs,
        handlers::runs::get_active_run,
        handlers::runs::get_run,
        handlers::runs::get_run_metrics,
        handlers::runs::start_run,
        handlers::runs::stop_run,
        handlers::runs::delete_runs,
        // Interaction
        handlers::interaction::get_interaction,
        handlers::interaction::post_stream,
        handlers::interaction::list_stream_messages,
        handlers::interaction::register_input,
        handlers::interaction::emit_input_event,
        handlers::interaction::claim_input_events,
        handlers::interaction::serve_artifact_file,
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    let home = dm_core::config::resolve_home(None).expect("Failed to resolve dm home");

    let events = EventStore::open(&home).expect("Failed to open event store");

    let state = AppState {
        home: Arc::new(home),
        events: Arc::new(events),
        interaction_events: broadcast::channel(512).0,
        input_events: broadcast::channel(512).0,
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
            "/api/dataflows/{name}/config-schema",
            get(handlers::get_dataflow_config_schema),
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
        .route("/api/runs/{id}/view", get(handlers::get_run_view))
        .route("/api/runs/delete", post(handlers::delete_runs))
        .route("/api/runs/{id}/logs/{node_id}", get(handlers::get_run_logs))
        .route(
            "/api/runs/{id}/logs/{node_id}/tail",
            get(handlers::tail_run_logs),
        )
        .route("/api/runs/{id}/interaction", get(handlers::get_interaction))
        .route(
            "/api/runs/{id}/interaction/ws",
            get(handlers::interaction_ws),
        )
        .route(
            "/api/runs/{id}/interaction/stream/messages",
            get(handlers::list_stream_messages),
        )
        .route(
            "/api/runs/{id}/interaction/stream",
            post(handlers::post_stream),
        )
        .route(
            "/api/runs/{id}/interaction/input/register",
            post(handlers::register_input),
        )
        .route(
            "/api/runs/{id}/interaction/input/events",
            post(handlers::emit_input_event),
        )
        .route(
            "/api/runs/{id}/interaction/input/claim/{node_id}",
            get(handlers::claim_input_events),
        )
        .route(
            "/api/runs/{id}/interaction/input/ws/{node_id}",
            get(handlers::input_ws),
        )
        .route(
            "/api/runs/{id}/artifacts/{*path}",
            get(handlers::serve_artifact_file),
        )
        .route("/api/runs/{id}/ws", get(handlers::run_ws))
        // ─── Events / Observability ───
        .route("/api/events/count", get(handlers::count_events))
        .route("/api/events/export", get(handlers::export_events))
        .route("/api/events", get(handlers::query_events))
        .route("/api/events", post(handlers::ingest_event))
        // ─── Middleware ───
        .layer(CorsLayer::permissive())
        .with_state(state.clone())
        // ─── Swagger UI ───
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
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
