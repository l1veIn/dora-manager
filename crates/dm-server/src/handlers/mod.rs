mod dataflow;
mod events;
mod nodes;
mod panel;
mod runs;
mod runtime;
mod system;
mod web;

use axum::http::StatusCode;
use axum::response::IntoResponse;

pub use dataflow::{
    delete_dataflow, get_dataflow, list_dataflows, save_dataflow, start_dataflow, stop_dataflow,
};
pub use events::{count_events, export_events, ingest_event, query_events};
pub use nodes::{
    create_node, get_node_config, import_node, install_node, list_nodes, node_readme, node_status,
    save_node_config, uninstall_node,
};
pub use panel::{list_sessions_panel, query_assets, send_command, serve_asset_file};
pub use runs::{delete_run, get_run, get_run_logs, list_runs};
pub use runtime::{down, install, uninstall, up, use_version};
pub use system::{doctor, get_config, status, update_config, versions};
pub use web::serve_web;

pub(crate) fn err(e: impl std::fmt::Display) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
