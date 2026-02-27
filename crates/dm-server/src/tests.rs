use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Query, State};
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
        Json(handlers::ConfigUpdate {
            active_version: Some("0.4.1".to_string()),
        }),
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
async fn list_nodes_returns_empty_array() {
    let (_tmp, state) = test_state();

    let resp = handlers::list_nodes(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let nodes: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert!(nodes.is_empty());
}

#[tokio::test]
async fn uninstall_returns_bad_request_for_missing_version() {
    let (_tmp, state) = test_state();

    let resp = handlers::uninstall(
        State(state),
        Json(handlers::UninstallRequest {
            version: "9.9.9".to_string(),
        }),
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
        Json(handlers::UseRequest {
            version: "0.4.1".to_string(),
        }),
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
        Json(handlers::UninstallNodeRequest {
            id: "missing-node".to_string(),
        }),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("missing-node"));
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
    assert_eq!(up_json["message"], "started");

    let down_resp = handlers::down(State(state)).await.into_response();
    assert_eq!(down_resp.status(), axum::http::StatusCode::OK);
    let down_body = body_text(down_resp).await;
    let down_json: serde_json::Value = serde_json::from_str(&down_body).unwrap();
    assert_eq!(down_json["success"], true);
    assert_eq!(down_json["message"], "stopped");
}

#[tokio::test]
async fn ingest_and_query_events_roundtrip() {
    let (_tmp, state) = test_state();

    let event = dm_core::events::EventBuilder::new(dm_core::events::EventSource::Frontend, "ui.click")
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
    let content_type = resp.headers().get(axum::http::header::CONTENT_TYPE).unwrap();
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
        Json(handlers::RunDataflowRequest {
            yaml: "nodes: [".to_string(),
        }),
    )
    .await
    .into_response();

    // No dora binary configured, so runtime check fails → 409 Conflict
    assert_eq!(resp.status(), axum::http::StatusCode::CONFLICT);
    let body = body_text(resp).await;
    assert!(body.contains("not running"));
}
