use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Query, State};
use axum::http::Uri;
use axum::response::IntoResponse;
use axum::Json;
use tempfile::TempDir;

use crate::handlers;
use crate::AppState;

fn test_state() -> (TempDir, AppState) {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();
    let events = dm_core::events::EventStore::open(&home).unwrap();
    let state = AppState {
        home: Arc::new(home),
        events: Arc::new(events),
    };
    (tmp, state)
}

fn setup_fake_dora_home(home: &std::path::Path, active_version: &str) {
    let version_dir = dm_core::config::versions_dir(home).join(active_version);
    std::fs::create_dir_all(&version_dir).unwrap();

    let bin = version_dir.join("dora");
    std::fs::write(
        &bin,
        r#"#!/bin/sh
cmd="$1"
case "$cmd" in
  --version)
    echo "dora-cli 0.4.1"
    ;;
  check)
    echo "Runtime OK"
    ;;
  list)
    echo "flow-a"
    echo "flow-b"
    ;;
  up)
    echo "started"
    ;;
  destroy)
    echo "stopped"
    ;;
  start)
    echo "dataflow started"
    ;;
  stop)
    echo "dataflow stopped"
    ;;
  *)
    echo "unknown command: $cmd" >&2
    exit 1
    ;;
esac
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();
    }

    dm_core::config::save_config(
        home,
        &dm_core::config::DmConfig {
            active_version: Some(active_version.to_string()),
        },
    )
    .unwrap();
}

fn setup_installed_node(home: &std::path::Path, id: &str) {
    setup_node_with_build(home, id, "python");
}

fn setup_node_with_build(home: &std::path::Path, id: &str, build: &str) {
    let node_dir = dm_core::node::node_dir(home, id);
    std::fs::create_dir_all(&node_dir).unwrap();
    let meta = dm_core::node::NodeMetaFile {
        id: id.to_string(),
        name: String::new(),
        version: "1.0.0".to_string(),
        installed_at: "1234567890".to_string(),
        source: dm_core::node::NodeSource {
            build: build.to_string(),
            github: None,
        },
        description: String::new(),
        executable: String::new(),
        author: None,
        category: String::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        avatar: None,
        config_schema: None,
    };
    std::fs::write(
        dm_core::node::dm_json_path(home, id),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();
}

async fn body_text(resp: axum::response::Response) -> String {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn get_config_returns_default_config() {
    let (_tmp, state) = test_state();

    let resp = handlers::get_config(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.get("active_version").is_some());
    assert!(json["active_version"].is_null());
}

#[tokio::test]
async fn doctor_handler_returns_ok_json() {
    let (_tmp, state) = test_state();

    let resp = handlers::doctor(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.get("python").is_some());
    assert!(json.get("uv").is_some());
    assert!(json.get("rust").is_some());
}

#[tokio::test]
async fn update_config_persists_active_version() {
    let (_tmp, state) = test_state();

    let resp = handlers::update_config(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "active_version": "0.4.1"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let cfg = dm_core::config::load_config(&state.home).unwrap();
    assert_eq!(cfg.active_version.as_deref(), Some("0.4.1"));
}

#[tokio::test]
async fn status_handler_uses_fake_dora_binary() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home(&state.home, "0.4.1");

    let resp = handlers::status(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["active_version"], "0.4.1");
    assert_eq!(json["actual_version"], "0.4.1");
    assert_eq!(json["runtime_running"], true);
    assert_eq!(json["runtime_output"], "Runtime OK");
    assert_eq!(json["dataflows"][0], "flow-a");
    assert_eq!(json["dataflows"][1], "flow-b");
}

#[tokio::test]
async fn node_status_returns_404_for_missing_node() {
    let (_tmp, state) = test_state();

    let resp = handlers::node_status(State(state), Path("missing-node".to_string()))
        .await
        .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);

    let body = body_text(resp).await;
    assert!(body.contains("missing-node"));
}

#[tokio::test]
async fn node_status_returns_entry_for_installed_node() {
    let (_tmp, state) = test_state();
    setup_installed_node(&state.home, "demo-node");

    let resp = handlers::node_status(State(state), Path("demo-node".to_string()))
        .await
        .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["id"], "demo-node");
    assert_eq!(json["version"], "1.0.0");
}

#[tokio::test]
async fn list_nodes_returns_empty_array() {
    let (_tmp, state) = test_state();

    let resp = handlers::list_nodes(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let nodes: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert!(nodes.is_empty());
}

#[tokio::test]
async fn list_dataflows_returns_empty_array() {
    let (_tmp, state) = test_state();

    let resp = handlers::list_dataflows(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let dataflows: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert!(dataflows.is_empty());
}

#[tokio::test]
async fn dataflow_crud_handlers_roundtrip() {
    let (_tmp, state) = test_state();

    let save_resp = handlers::save_dataflow(
        State(state.clone()),
        Path("demo-flow".to_string()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(save_resp.status(), axum::http::StatusCode::OK);

    let list_resp = handlers::list_dataflows(State(state.clone()))
        .await
        .into_response();
    assert_eq!(list_resp.status(), axum::http::StatusCode::OK);
    let list_body = body_text(list_resp).await;
    let dataflows: Vec<serde_json::Value> = serde_json::from_str(&list_body).unwrap();
    assert_eq!(dataflows.len(), 1);
    assert_eq!(dataflows[0]["name"], "demo-flow");

    let get_resp = handlers::get_dataflow(State(state.clone()), Path("demo-flow".to_string()))
        .await
        .into_response();
    assert_eq!(get_resp.status(), axum::http::StatusCode::OK);
    let get_body = body_text(get_resp).await;
    let get_json: serde_json::Value = serde_json::from_str(&get_body).unwrap();
    assert_eq!(get_json["yaml"], "nodes: []");

    let delete_resp =
        handlers::delete_dataflow(State(state.clone()), Path("demo-flow".to_string()))
            .await
            .into_response();
    assert_eq!(delete_resp.status(), axum::http::StatusCode::OK);

    let missing_resp = handlers::get_dataflow(State(state), Path("demo-flow".to_string()))
        .await
        .into_response();
    assert_eq!(missing_resp.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn uninstall_returns_bad_request_for_missing_version() {
    let (_tmp, state) = test_state();

    let resp = handlers::uninstall(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "version": "9.9.9"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("not installed"));
}

#[tokio::test]
async fn use_version_returns_bad_request_for_missing_version() {
    let (_tmp, state) = test_state();

    let resp = handlers::use_version(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "version": "0.4.1"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("not installed"));
}

#[tokio::test]
async fn uninstall_node_returns_bad_request_for_missing_node() {
    let (_tmp, state) = test_state();

    let resp = handlers::uninstall_node(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "id": "missing-node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("missing-node"));
}

#[tokio::test]
async fn uninstall_node_returns_success_for_existing_node() {
    let (_tmp, state) = test_state();
    setup_installed_node(&state.home, "demo-node");

    let resp = handlers::uninstall_node(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "id": "demo-node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    assert!(!dm_core::node::node_dir(&state.home, "demo-node").exists());
}

#[tokio::test]
async fn create_node_returns_success_and_duplicate_returns_bad_request() {
    let (_tmp, state) = test_state();

    let create_resp = handlers::create_node(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "id": "new-node",
                "description": "test node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(create_resp.status(), axum::http::StatusCode::OK);

    let duplicate_resp = handlers::create_node(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "id": "new-node",
                "description": "test node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(duplicate_resp.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn node_config_handlers_roundtrip() {
    let (_tmp, state) = test_state();
    let home = state.home.clone();
    dm_core::node::create_node(&home, "cfg-node", "configurable").unwrap();

    let save_resp = handlers::save_node_config(
        State(state.clone()),
        Path("cfg-node".to_string()),
        Json(serde_json::json!({ "threshold": 0.9 })),
    )
    .await
    .into_response();
    assert_eq!(save_resp.status(), axum::http::StatusCode::OK);

    let get_resp = handlers::get_node_config(State(state), Path("cfg-node".to_string()))
        .await
        .into_response();
    assert_eq!(get_resp.status(), axum::http::StatusCode::OK);

    let body = body_text(get_resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["threshold"], 0.9);
}

#[tokio::test]
async fn save_node_config_returns_bad_request_for_missing_node() {
    let (_tmp, state) = test_state();

    let resp = handlers::save_node_config(
        State(state),
        Path("missing-node".to_string()),
        Json(serde_json::json!({ "threshold": 0.5 })),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn install_node_returns_bad_request_for_missing_node() {
    let (_tmp, state) = test_state();

    let resp = handlers::install_node(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "id": "missing-node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn install_node_returns_bad_request_for_unsupported_build() {
    let (_tmp, state) = test_state();
    setup_node_with_build(&state.home, "bad-node", "npm install bad-node");

    let resp = handlers::install_node(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "id": "bad-node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("Unsupported build type"));
}

#[tokio::test]
async fn download_node_returns_bad_request_for_existing_directory() {
    let (_tmp, state) = test_state();
    std::fs::create_dir_all(dm_core::node::node_dir(&state.home, "demo-node")).unwrap();

    let resp = handlers::download_node(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "id": "demo-node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("already exists"));
}

#[tokio::test]
async fn stop_dataflow_returns_500_without_active_dora() {
    let (_tmp, state) = test_state();

    let resp = handlers::stop_dataflow(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);

    let body = body_text(resp).await;
    assert!(body.contains("No active dora version"));
}

#[tokio::test]
async fn up_and_down_handlers_use_fake_dora_binary() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home(&state.home, "0.4.1");

    let up_resp = handlers::up(State(state.clone())).await.into_response();
    assert_eq!(up_resp.status(), axum::http::StatusCode::OK);
    let up_body = body_text(up_resp).await;
    let up_json: serde_json::Value = serde_json::from_str(&up_body).unwrap();
    assert_eq!(up_json["success"], true);
    assert!(up_json["message"]
        .as_str()
        .unwrap_or_default()
        .contains("started"));

    let down_resp = handlers::down(State(state)).await.into_response();
    assert_eq!(down_resp.status(), axum::http::StatusCode::OK);
    let down_body = body_text(down_resp).await;
    let down_json: serde_json::Value = serde_json::from_str(&down_body).unwrap();
    assert_eq!(down_json["success"], false);
    assert!(down_json["message"]
        .as_str()
        .unwrap_or_default()
        .contains("still running"));
}

#[tokio::test]
async fn ingest_and_query_events_roundtrip() {
    let (_tmp, state) = test_state();

    let event =
        dm_core::events::EventBuilder::new(dm_core::events::EventSource::Frontend, "ui.click")
            .case_id("session_test")
            .message("clicked")
            .attr("button", "run")
            .build();

    let ingest_resp = handlers::ingest_event(State(state.clone()), Json(event))
        .await
        .into_response();
    assert_eq!(ingest_resp.status(), axum::http::StatusCode::OK);

    let query_resp = handlers::query_events(
        State(state),
        Query(dm_core::events::EventFilter {
            case_id: Some("session_test".to_string()),
            ..Default::default()
        }),
    )
    .await
    .into_response();
    assert_eq!(query_resp.status(), axum::http::StatusCode::OK);

    let body = body_text(query_resp).await;
    let events: Vec<dm_core::events::Event> = serde_json::from_str(&body).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].activity, "ui.click");
    assert_eq!(events[0].source, "frontend");
}

#[tokio::test]
async fn count_events_returns_count() {
    let (_tmp, state) = test_state();

    let event = dm_core::events::EventBuilder::new(dm_core::events::EventSource::Core, "doctor")
        .case_id("session_count")
        .build();
    let _ = handlers::ingest_event(State(state.clone()), Json(event))
        .await
        .into_response();

    let resp = handlers::count_events(
        State(state),
        Query(dm_core::events::EventFilter {
            case_id: Some("session_count".to_string()),
            ..Default::default()
        }),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["count"], 1);
}

#[tokio::test]
async fn export_events_returns_xml() {
    let (_tmp, state) = test_state();

    let event = dm_core::events::EventBuilder::new(dm_core::events::EventSource::Core, "doctor")
        .case_id("session_export")
        .message("OK")
        .build();
    state.events.emit(&event).unwrap();

    let resp = handlers::export_events(
        State(state),
        Query(dm_core::events::EventFilter {
            case_id: Some("session_export".to_string()),
            ..Default::default()
        }),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap();
    assert_eq!(content_type, "application/xml");

    let body = body_text(resp).await;
    assert!(body.contains("<log"));
    assert!(body.contains("doctor"));
    assert!(body.contains("session_export"));
}

#[tokio::test]
async fn query_events_returns_empty_list_when_no_match() {
    let (_tmp, state) = test_state();

    let resp = handlers::query_events(
        State(state),
        Query(dm_core::events::EventFilter {
            case_id: Some("missing_case".to_string()),
            ..Default::default()
        }),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = body_text(resp).await;
    let events: Vec<dm_core::events::Event> = serde_json::from_str(&body).unwrap();
    assert!(events.is_empty());
}

#[tokio::test]
async fn start_dataflow_returns_conflict_without_runtime() {
    let (_tmp, state) = test_state();

    let resp = handlers::start_dataflow(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: ["
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    // No dora binary configured, so runtime check fails → 409 Conflict
    assert_eq!(resp.status(), axum::http::StatusCode::CONFLICT);
    let body = body_text(resp).await;
    assert!(body.contains("not running"));
}

#[tokio::test]
async fn start_dataflow_returns_error_for_invalid_yaml_when_runtime_is_up() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home(&state.home, "0.4.1");

    let resp = handlers::start_dataflow(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: ["
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let body = body_text(resp).await;
    assert!(body.contains("Failed to parse yaml"));
}

#[tokio::test]
async fn serve_web_root_returns_index_html() {
    let resp = handlers::serve_web(Uri::from_static("/"))
        .await
        .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap();
    assert!(content_type
        .to_str()
        .unwrap_or_default()
        .contains("text/html"));

    let body = body_text(resp).await;
    assert!(body.contains("<!doctype html>") || body.contains("<html"));
}

#[tokio::test]
async fn serve_web_unknown_path_falls_back_to_index() {
    let resp = handlers::serve_web(Uri::from_static("/missing-route"))
        .await
        .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap();
    assert!(content_type
        .to_str()
        .unwrap_or_default()
        .contains("text/html"));
}
