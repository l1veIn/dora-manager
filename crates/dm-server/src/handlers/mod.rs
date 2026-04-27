pub(crate) mod bridge_socket;
pub(crate) mod dataflow;
pub(crate) mod events;
pub(crate) mod messages;
pub(crate) mod nodes;
pub(crate) mod run_ws;
pub(crate) mod runs;
pub(crate) mod runtime;
pub(crate) mod services;
pub(crate) mod system;
pub(crate) mod web;

use axum::http::StatusCode;
use axum::response::IntoResponse;

pub use dataflow::{
    delete_dataflow, get_dataflow, get_dataflow_config_schema, get_dataflow_history_version,
    get_dataflow_meta, get_dataflow_view, import_dataflows, inspect_dataflow,
    list_dataflow_history, list_dataflows, restore_dataflow_history_version, save_dataflow,
    save_dataflow_meta, save_dataflow_view, start_dataflow, stop_dataflow,
};
pub use events::{count_events, export_events, ingest_event, query_events};
pub use messages::{
    get_interaction, get_snapshots, get_stream, list_messages, list_streams, messages_ws, node_ws,
    push_message, serve_artifact_file,
};
pub use nodes::{
    create_node, get_node_config, get_node_file_content, get_node_files, import_node, install_node,
    list_nodes, node_readme, node_status, open_node, save_node_config, serve_node_artifact_file,
    uninstall_node,
};
pub use run_ws::run_ws;
pub use runs::{
    delete_runs, get_active_run, get_run, get_run_dataflow, get_run_logs, get_run_metrics,
    get_run_transpiled, get_run_view, list_runs, start_run, stop_run, stream_run_logs,
    tail_run_logs,
};
pub use runtime::{down, install, uninstall, up, use_version};
pub use services::{
    create_service, get_service_config, get_service_file_content, get_service_files,
    import_service, install_service, list_services, open_service, save_service_config,
    serve_service_artifact_file, service_readme, service_status, uninstall_service,
};
pub use system::{
    doctor, get_config, install_media, media_status, status, update_config, versions,
};
pub use web::serve_web;

pub(crate) fn err(e: impl std::fmt::Display) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
