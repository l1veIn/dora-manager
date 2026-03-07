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
    delete_dataflow, get_dataflow, get_dataflow_config, get_dataflow_config_schema,
    get_dataflow_history_version, get_dataflow_meta, import_dataflows, inspect_dataflow,
    list_dataflow_history, list_dataflows, restore_dataflow_history_version, save_dataflow,
    save_dataflow_config, save_dataflow_meta, start_dataflow, stop_dataflow,
};
pub use events::{count_events, export_events, ingest_event, query_events};
pub use nodes::{
    create_node, get_node_config, get_node_file_content, get_node_files, import_node,
    install_node, list_nodes, node_readme, node_status, save_node_config, uninstall_node,
};
pub use panel::{query_assets, send_command, serve_asset_file};
pub use runs::{
    delete_runs, get_active_run, get_run, get_run_dataflow, get_run_logs, get_run_transpiled,
    list_runs, start_run, stop_run, tail_run_logs,
};
pub use runtime::{down, install, uninstall, up, use_version};
pub use system::{doctor, get_config, status, update_config, versions};
pub use web::serve_web;

pub(crate) fn err(e: impl std::fmt::Display) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
