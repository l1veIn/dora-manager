use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Query, State};
use axum::http::Uri;
use axum::response::IntoResponse;
use axum::Json;
use tempfile::TempDir;

use crate::handlers;
use crate::AppState;

const FAKE_DORA_UUID: &str = "019cc181-adad-7654-aa78-63502362337b";

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
        format!(
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
    echo "UUID Name Status Nodes CPU Memory"
    echo "019cc181-adad-7654-aa78-63502362337b flow-a Running 1 0.0% 0.0"
    echo "019cc181-adad-7654-aa78-635023623380 flow-b Succeeded 2 0.0% 0.0"
    ;;
  up)
    echo "started"
    ;;
  destroy)
    echo "stopped"
    ;;
  start)
    echo "dataflow started: {fake_uuid}"
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
            fake_uuid = FAKE_DORA_UUID,
        ),
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

fn setup_fake_dora_home_with_active_file(
    home: &std::path::Path,
    active_version: &str,
) -> std::path::PathBuf {
    let version_dir = dm_core::config::versions_dir(home).join(active_version);
    std::fs::create_dir_all(&version_dir).unwrap();
    let state_file = home.join("active_dataflow_id");

    let bin = version_dir.join("dora");
    std::fs::write(
        &bin,
        format!(
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
    if [ -f "{state_file}" ]; then
      echo "UUID Name Status Nodes CPU Memory"
      printf "%s test-flow Running 1 0.0%% 0.0\\ GB\\n" "$(cat "{state_file}")"
    fi
    ;;
  start)
    run_yaml="$2"
    run_dir="$(dirname "$run_yaml")"
    mkdir -p "$run_dir/out/{fake_uuid}"
    echo "worker log line" > "$run_dir/out/{fake_uuid}/log_worker.txt"
    echo "{fake_uuid}" > "{state_file}"
    echo "dataflow started: {fake_uuid}"
    ;;
  stop)
    rm -f "{state_file}"
    echo "dataflow stopped"
    ;;
  *)
    exit 0
    ;;
esac
"#,
            state_file = state_file.display(),
            fake_uuid = FAKE_DORA_UUID
        ),
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

    state_file
}

fn setup_installed_node(home: &std::path::Path, id: &str) {
    setup_node_with_build(home, id, "python");
}

fn setup_node_with_build(home: &std::path::Path, id: &str, build: &str) {
    let node_dir = dm_core::node::node_dir(home, id);
    std::fs::create_dir_all(&node_dir).unwrap();
    let meta = dm_core::node::Node {
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
        repository: None,
        maintainers: Vec::new(),
        license: None,
        display: dm_core::node::NodeDisplay::default(),
        capabilities: Vec::new(),
        runtime: dm_core::node::NodeRuntime::default(),
        ports: Vec::new(),
        files: dm_core::node::NodeFiles::default(),
        examples: Vec::new(),
        config_schema: None,
        path: Default::default(),
    };
    std::fs::write(
        dm_core::node::dm_json_path(home, id),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();
}

fn setup_panel_run(home: &std::path::Path, run_id: &str) {
    dm_core::runs::create_layout(home, run_id).unwrap();
    let run = dm_core::runs::RunInstance {
        run_id: run_id.to_string(),
        dataflow_name: "panel-demo".to_string(),
        started_at: "2026-03-06T00:00:00Z".to_string(),
        has_panel: true,
        ..Default::default()
    };
    dm_core::runs::save_run(home, &run).unwrap();
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
    assert!(json["active_runs"].as_array().unwrap().is_empty());
    assert!(json["recent_runs"].as_array().unwrap().is_empty());
    assert!(json["dora_probe"].as_array().unwrap().is_empty());
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
async fn list_nodes_returns_builtin_entries() {
    let (_tmp, state) = test_state();

    let resp = handlers::list_nodes(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let nodes: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert!(nodes
        .iter()
        .any(|node| node["id"] == "dm-test-media-capture"));
    assert!(nodes
        .iter()
        .any(|node| node["id"] == "dm-test-audio-capture"));
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
    assert_eq!(dataflows[0]["meta"]["name"], "demo-flow");
    assert_eq!(dataflows[0]["executable"]["can_run"], true);

    let get_resp = handlers::get_dataflow(State(state.clone()), Path("demo-flow".to_string()))
        .await
        .into_response();
    assert_eq!(get_resp.status(), axum::http::StatusCode::OK);
    let get_body = body_text(get_resp).await;
    let get_json: serde_json::Value = serde_json::from_str(&get_body).unwrap();
    assert_eq!(get_json["yaml"], "nodes: []");
    assert_eq!(get_json["meta"]["name"], "demo-flow");
    assert_eq!(get_json["executable"]["can_run"], true);

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
async fn dataflow_meta_and_config_handlers_roundtrip() {
    let (_tmp, state) = test_state();

    let _ = handlers::save_dataflow(
        State(state.clone()),
        Path("demo-flow".to_string()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []\n"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    let meta_resp = handlers::save_dataflow_meta(
        State(state.clone()),
        Path("demo-flow".to_string()),
        Json(dm_core::dataflow::FlowMeta {
            id: "demo-flow".to_string(),
            name: "Demo Flow".to_string(),
            description: "Demo".to_string(),
            r#type: "chat".to_string(),
            tags: vec!["llm".to_string()],
            author: Some("yangchen".to_string()),
            cover: None,
            created_at: String::new(),
            updated_at: String::new(),
        }),
    )
    .await
    .into_response();
    assert_eq!(meta_resp.status(), axum::http::StatusCode::OK);

    let get_meta_resp =
        handlers::get_dataflow_meta(State(state.clone()), Path("demo-flow".to_string()))
            .await
            .into_response();
    assert_eq!(get_meta_resp.status(), axum::http::StatusCode::OK);
    let meta_body = body_text(get_meta_resp).await;
    let meta_json: serde_json::Value = serde_json::from_str(&meta_body).unwrap();
    assert_eq!(meta_json["name"], "Demo Flow");
    assert_eq!(meta_json["type"], "chat");

    let config_resp = handlers::save_dataflow_config(
        State(state.clone()),
        Path("demo-flow".to_string()),
        Json(serde_json::json!({
            "dora-qwen": {
                "model": "qwen-max"
            }
        })),
    )
    .await
    .into_response();
    assert_eq!(config_resp.status(), axum::http::StatusCode::OK);

    let get_config_resp =
        handlers::get_dataflow_config(State(state), Path("demo-flow".to_string()))
            .await
            .into_response();
    assert_eq!(get_config_resp.status(), axum::http::StatusCode::OK);
    let config_body = body_text(get_config_resp).await;
    let config_json: serde_json::Value = serde_json::from_str(&config_body).unwrap();
    assert_eq!(config_json["config"]["dora-qwen"]["model"], "qwen-max");
    assert_eq!(config_json["executable"]["can_run"], true);
}

#[tokio::test]
async fn dataflow_history_handlers_roundtrip() {
    let (_tmp, state) = test_state();

    let _ = handlers::save_dataflow(
        State(state.clone()),
        Path("demo-flow".to_string()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []\n"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    let _ = handlers::save_dataflow(
        State(state.clone()),
        Path("demo-flow".to_string()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes:\n  - id: a\n"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    let history_resp =
        handlers::list_dataflow_history(State(state.clone()), Path("demo-flow".to_string()))
            .await
            .into_response();
    assert_eq!(history_resp.status(), axum::http::StatusCode::OK);
    let history_body = body_text(history_resp).await;
    let history: Vec<serde_json::Value> = serde_json::from_str(&history_body).unwrap();
    assert_eq!(history.len(), 1);
    let version = history[0]["version"].as_str().unwrap().to_string();

    let version_resp = handlers::get_dataflow_history_version(
        State(state.clone()),
        Path(("demo-flow".to_string(), version.clone())),
    )
    .await
    .into_response();
    assert_eq!(version_resp.status(), axum::http::StatusCode::OK);
    let version_body = body_text(version_resp).await;
    let version_json: serde_json::Value = serde_json::from_str(&version_body).unwrap();
    assert_eq!(version_json["yaml"], "nodes: []\n");

    let restore_resp = handlers::restore_dataflow_history_version(
        State(state.clone()),
        Path(("demo-flow".to_string(), version)),
    )
    .await
    .into_response();
    assert_eq!(restore_resp.status(), axum::http::StatusCode::OK);

    let get_resp = handlers::get_dataflow(State(state), Path("demo-flow".to_string()))
        .await
        .into_response();
    let get_body = body_text(get_resp).await;
    let get_json: serde_json::Value = serde_json::from_str(&get_body).unwrap();
    assert_eq!(get_json["yaml"], "nodes: []\n");
}

#[tokio::test]
async fn import_dataflows_handler_imports_local_yaml() {
    let (_tmp, state) = test_state();
    let source = state.home.join("external.yml");
    std::fs::write(&source, "nodes: []\n").unwrap();

    let resp = handlers::import_dataflows(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "sources": [source.display().to_string()]
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["imported"][0]["name"], "external");
    assert_eq!(json["imported"][0]["executable"]["can_run"], true);
    assert_eq!(
        std::fs::read_to_string(state.home.join("dataflows/external/dataflow.yml")).unwrap(),
        "nodes: []\n"
    );
}

#[tokio::test]
async fn get_dataflow_config_schema_returns_aggregated_fields() {
    let (_tmp, state) = test_state();
    setup_installed_node(&state.home, "cfg-node");

    let node_dir = dm_core::node::node_dir(&state.home, "cfg-node");
    let mut meta: dm_core::node::Node = serde_json::from_str(
        &std::fs::read_to_string(dm_core::node::dm_json_path(&state.home, "cfg-node")).unwrap(),
    )
    .unwrap();
    meta.config_schema = Some(serde_json::json!({
        "mode": {
            "default": "default-mode",
            "x-widget": { "type": "select", "options": ["default-mode", "flow-mode"] }
        }
    }));
    std::fs::write(
        node_dir.join("dm.json"),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();

    let _ = handlers::save_dataflow(
        State(state.clone()),
        Path("cfg-flow".to_string()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes:\n  - id: worker\n    node: cfg-node\n"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    let _ = handlers::save_dataflow_config(
        State(state.clone()),
        Path("cfg-flow".to_string()),
        Json(serde_json::json!({
            "worker": { "mode": "flow-mode" }
        })),
    )
    .await
    .into_response();

    let resp = handlers::get_dataflow_config_schema(State(state), Path("cfg-flow".to_string()))
        .await
        .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["nodes"][0]["yaml_id"], "worker");
    assert_eq!(
        json["nodes"][0]["fields"]["mode"]["effective_source"],
        "flow"
    );
    assert_eq!(
        json["nodes"][0]["fields"]["mode"]["effective_value"],
        "flow-mode"
    );
    assert_eq!(json["executable"]["can_run"], true);
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
async fn import_node_imports_local_directory_with_inferred_id() {
    let (_tmp, state) = test_state();
    let source_dir = state.home.join("imports/relative-node");
    std::fs::create_dir_all(source_dir.join("pkg")).unwrap();
    std::fs::write(source_dir.join("README.md"), "# Relative Node\n").unwrap();
    std::fs::write(
        source_dir.join("pyproject.toml"),
        "[project]\nname = \"relative-node\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(source_dir.join("pkg/__init__.py"), "").unwrap();

    let resp = handlers::import_node(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "source": "imports/relative-node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["id"], "relative-node");
    assert!(dm_core::node::node_dir(&state.home, "relative-node").exists());
}

#[tokio::test]
async fn import_node_returns_bad_request_for_missing_source() {
    let (_tmp, state) = test_state();

    let resp = handlers::import_node(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "source": "imports/missing-node"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("not found"));
}

#[tokio::test]
async fn node_readme_returns_local_content_and_fallback_message() {
    let (_tmp, state) = test_state();
    dm_core::node::create_node(&state.home, "docs-node", "Readable").unwrap();

    let ok_resp = handlers::node_readme(State(state.clone()), Path("docs-node".to_string()))
        .await
        .into_response();
    assert_eq!(ok_resp.status(), axum::http::StatusCode::OK);
    let ok_body = body_text(ok_resp).await;
    assert!(ok_body.contains("# docs-node"));

    let missing_resp = handlers::node_readme(State(state), Path("missing-node".to_string()))
        .await
        .into_response();
    assert_eq!(missing_resp.status(), axum::http::StatusCode::OK);
    let missing_body = body_text(missing_resp).await;
    assert!(missing_body.contains("No README found locally"));
}

#[tokio::test]
async fn node_file_handlers_return_tree_and_file_content() {
    let (_tmp, state) = test_state();
    dm_core::node::create_node(&state.home, "file-node", "Files").unwrap();
    let node_dir = dm_core::node::node_dir(&state.home, "file-node");
    std::fs::create_dir_all(node_dir.join("nested")).unwrap();
    std::fs::write(node_dir.join("nested/config.yaml"), "name: file-node\n").unwrap();

    let files_resp = handlers::get_node_files(State(state.clone()), Path("file-node".to_string()))
        .await
        .into_response();
    assert_eq!(files_resp.status(), axum::http::StatusCode::OK);
    let files_body = body_text(files_resp).await;
    let files: Vec<String> = serde_json::from_str(&files_body).unwrap();
    assert!(files.iter().any(|path| path == "README.md"));
    assert!(files.iter().any(|path| path == "nested/config.yaml"));

    let content_resp = handlers::get_node_file_content(
        State(state),
        Path(("file-node".to_string(), "nested/config.yaml".to_string())),
    )
    .await
    .into_response();
    assert_eq!(content_resp.status(), axum::http::StatusCode::OK);
    let content_body = body_text(content_resp).await;
    assert_eq!(content_body, "name: file-node\n");
}

#[tokio::test]
async fn node_file_handlers_map_traversal_and_missing_paths() {
    let (_tmp, state) = test_state();
    dm_core::node::create_node(&state.home, "file-node", "Files").unwrap();

    let bad_resp = handlers::get_node_file_content(
        State(state.clone()),
        Path(("file-node".to_string(), "../secret.txt".to_string())),
    )
    .await
    .into_response();
    assert_eq!(bad_resp.status(), axum::http::StatusCode::BAD_REQUEST);

    let missing_resp = handlers::get_node_file_content(
        State(state),
        Path(("missing-node".to_string(), "README.md".to_string())),
    )
    .await
    .into_response();
    assert_eq!(missing_resp.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stop_dataflow_returns_404_without_active_run() {
    let (_tmp, state) = test_state();

    let resp = handlers::stop_dataflow(State(state)).await.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);

    let body = body_text(resp).await;
    assert!(body.contains("No active run found"));
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
async fn start_dataflow_returns_error_when_auto_up_fails() {
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

    // No dora binary configured, so auto-up fails → 500
    assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let body = body_text(resp).await;
    assert!(body.contains("auto-start"));
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
    assert!(body.contains("is not executable: invalid yaml"));
}

#[tokio::test]
async fn start_run_returns_conflict_for_same_active_dataflow() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home_with_active_file(&state.home, "0.4.1");

    let first = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(first.status(), axum::http::StatusCode::OK);

    let second = handlers::start_run(
        State(state),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(second.status(), axum::http::StatusCode::CONFLICT);

    let body = body_text(second).await;
    assert!(body.contains("already running as run"));
}

#[tokio::test]
async fn list_runs_refreshes_stale_running_status() {
    let (_tmp, state) = test_state();
    let active_file = setup_fake_dora_home_with_active_file(&state.home, "0.4.1");

    let started = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(started.status(), axum::http::StatusCode::OK);

    std::fs::remove_file(active_file).unwrap();

    let resp = handlers::list_runs(
        State(state),
        Query(serde_json::from_value(serde_json::json!({})).unwrap()),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["runs"][0]["status"], "stopped");
}

#[tokio::test]
async fn list_runs_supports_status_and_search_filters() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home_with_active_file(&state.home, "0.4.1");

    let started = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "search-demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(started.status(), axum::http::StatusCode::OK);

    let started_body = body_text(started).await;
    let started_json: serde_json::Value = serde_json::from_str(&started_body).unwrap();
    let run_id = started_json["run_id"].as_str().unwrap().to_string();

    let filtered = handlers::list_runs(
        State(state.clone()),
        Query(
            serde_json::from_value(serde_json::json!({
                "status": "running",
                "search": "search-demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(filtered.status(), axum::http::StatusCode::OK);

    let filtered_body = body_text(filtered).await;
    let filtered_json: serde_json::Value = serde_json::from_str(&filtered_body).unwrap();
    assert_eq!(filtered_json["total"], 1);
    assert_eq!(filtered_json["runs"][0]["id"], run_id);

    let empty = handlers::list_runs(
        State(state),
        Query(
            serde_json::from_value(serde_json::json!({
                "status": "failed",
                "search": "search-demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(empty.status(), axum::http::StatusCode::OK);
    let empty_body = body_text(empty).await;
    let empty_json: serde_json::Value = serde_json::from_str(&empty_body).unwrap();
    assert_eq!(empty_json["total"], 0);
}

#[tokio::test]
async fn get_active_run_returns_active_run_summaries() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home_with_active_file(&state.home, "0.4.1");

    let started = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "active-demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(started.status(), axum::http::StatusCode::OK);

    let active = handlers::get_active_run(
        State(state),
        Query(serde_json::from_value(serde_json::json!({})).unwrap()),
    ).await.into_response();
    assert_eq!(active.status(), axum::http::StatusCode::OK);

    let body = body_text(active).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.is_array());
    assert_eq!(json[0]["name"], "active-demo");
    assert_eq!(json[0]["status"], "running");
}

#[tokio::test]
async fn delete_runs_deletes_multiple_runs_via_post() {
    let (_tmp, state) = test_state();

    for run_id in ["run-a", "run-b"] {
        dm_core::runs::create_layout(&state.home, run_id).unwrap();
        let run = dm_core::runs::RunInstance {
            run_id: run_id.to_string(),
            dataflow_name: run_id.to_string(),
            started_at: "2026-03-06T00:00:00Z".to_string(),
            ..Default::default()
        };
        dm_core::runs::save_run(&state.home, &run).unwrap();
    }

    let resp = handlers::delete_runs(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "run_ids": ["run-a", "run-b"]
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["deleted_count"], 2);
    assert_eq!(json["failed_count"], 0);
    assert!(!dm_core::runs::run_dir(&state.home, "run-a").exists());
    assert!(!dm_core::runs::run_dir(&state.home, "run-b").exists());
}

#[tokio::test]
async fn tail_run_logs_returns_incremental_chunks() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home_with_active_file(&state.home, "0.4.1");

    let started = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(started.status(), axum::http::StatusCode::OK);

    let started_body = body_text(started).await;
    let started_json: serde_json::Value = serde_json::from_str(&started_body).unwrap();
    let run_id = started_json["run_id"].as_str().unwrap().to_string();

    let first = handlers::tail_run_logs(
        State(state.clone()),
        Path((run_id.clone(), "worker".to_string())),
        Query(serde_json::from_value(serde_json::json!({ "offset": 0 })).unwrap()),
    )
    .await
    .into_response();
    assert_eq!(first.status(), axum::http::StatusCode::OK);
    let first_body = body_text(first).await;
    let first_json: serde_json::Value = serde_json::from_str(&first_body).unwrap();
    assert_eq!(first_json["content"], "worker log line\n");
    let next_offset = first_json["next_offset"].as_u64().unwrap();
    assert!(next_offset > 0);

    let second = handlers::tail_run_logs(
        State(state),
        Path((run_id, "worker".to_string())),
        Query(serde_json::from_value(serde_json::json!({ "offset": next_offset })).unwrap()),
    )
    .await
    .into_response();
    assert_eq!(second.status(), axum::http::StatusCode::OK);
    let second_body = body_text(second).await;
    let second_json: serde_json::Value = serde_json::from_str(&second_body).unwrap();
    assert_eq!(second_json["content"], "");
    assert_eq!(second_json["next_offset"].as_u64().unwrap(), next_offset);
}

#[tokio::test]
async fn get_run_transpiled_returns_transpiled_snapshot() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home_with_active_file(&state.home, "0.4.1");

    let started = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": r#"
nodes:
  - id: worker
    path: python3
"#,
                "name": "demo"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(started.status(), axum::http::StatusCode::OK);

    let started_body = body_text(started).await;
    let started_json: serde_json::Value = serde_json::from_str(&started_body).unwrap();
    let run_id = started_json["run_id"].as_str().unwrap().to_string();

    let resp = handlers::get_run_transpiled(State(state), Path(run_id))
        .await
        .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = body_text(resp).await;
    assert!(body.contains("nodes:"));
    assert!(body.contains("worker"));
}

#[tokio::test]
async fn status_prefers_run_metadata_for_active_runs() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home_with_active_file(&state.home, "0.4.1");

    let started = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "demo-flow"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(started.status(), axum::http::StatusCode::OK);

    let status = handlers::status(State(state)).await.into_response();
    assert_eq!(status.status(), axum::http::StatusCode::OK);

    let body = body_text(status).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["active_runs"][0]["dataflow_name"], "demo-flow");
    assert_eq!(json["active_runs"][0]["status"], "running");
    assert_eq!(json["active_runs"][0]["expected_nodes"], 0);
    assert_eq!(json["dora_probe"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn query_panel_assets_rejects_run_without_panel() {
    let (_tmp, state) = test_state();
    setup_fake_dora_home(&state.home, "0.4.1");

    let started = handlers::start_run(
        State(state.clone()),
        Json(
            serde_json::from_value(serde_json::json!({
                "yaml": "nodes: []",
                "name": "demo-flow"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(started.status(), axum::http::StatusCode::OK);

    let started_body = body_text(started).await;
    let started_json: serde_json::Value = serde_json::from_str(&started_body).unwrap();
    let run_id = started_json["run_id"].as_str().unwrap().to_string();

    let resp = handlers::query_assets(
        State(state),
        Path(run_id),
        Query(serde_json::from_value(serde_json::json!({ "limit": 10 })).unwrap()),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("does not have a panel"));
}

#[tokio::test]
async fn query_panel_assets_returns_results_for_panel_run() {
    let (_tmp, state) = test_state();
    let run_id = "panel-assets";
    setup_panel_run(&state.home, run_id);

    let store = dm_core::runs::panel::PanelStore::open(&state.home, run_id).unwrap();
    store
        .write_asset("camera", "text/plain", b"first asset")
        .unwrap();

    let resp = handlers::query_assets(
        State(state),
        Path(run_id.to_string()),
        Query(serde_json::from_value(serde_json::json!({ "limit": 5 })).unwrap()),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = body_text(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["assets"].as_array().unwrap().len(), 1);
    assert_eq!(json["assets"][0]["input_id"], "camera");
}

#[tokio::test]
async fn serve_asset_file_rejects_traversal_and_serves_existing_files() {
    let (_tmp, state) = test_state();
    let run_id = "panel-files";
    setup_panel_run(&state.home, run_id);
    let panel_dir = dm_core::runs::run_panel_dir(&state.home, run_id);
    std::fs::create_dir_all(panel_dir.join("nested")).unwrap();
    std::fs::write(panel_dir.join("nested/result.json"), "{\"ok\":true}").unwrap();

    let bad_resp = handlers::serve_asset_file(
        State(state.clone()),
        Path((run_id.to_string(), "../secret.txt".to_string())),
    )
    .await
    .into_response();
    assert_eq!(bad_resp.status(), axum::http::StatusCode::BAD_REQUEST);

    let ok_resp = handlers::serve_asset_file(
        State(state),
        Path((run_id.to_string(), "nested/result.json".to_string())),
    )
    .await
    .into_response();
    assert_eq!(ok_resp.status(), axum::http::StatusCode::OK);
    assert_eq!(
        ok_resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/json"
    );
    let body = body_text(ok_resp).await;
    assert_eq!(body, "{\"ok\":true}");
}

#[tokio::test]
async fn serve_asset_file_returns_not_found_for_missing_file() {
    let (_tmp, state) = test_state();
    let run_id = "panel-missing";
    setup_panel_run(&state.home, run_id);

    let resp = handlers::serve_asset_file(
        State(state),
        Path((run_id.to_string(), "nested/missing.txt".to_string())),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn send_panel_command_accepts_command_shorthand() {
    let (_tmp, state) = test_state();

    let run_id = "panel-run";
    setup_panel_run(&state.home, run_id);

    let resp = handlers::send_command(
        State(state.clone()),
        Path(run_id.to_string()),
        Json(
            serde_json::from_value(serde_json::json!({
                "command": "{\"text\":\"hello\"}"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let store = dm_core::runs::panel::PanelStore::open(&state.home, run_id).unwrap();
    let mut since = 0;
    let commands = store.poll_commands(&mut since).unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].output_id, "input");
    assert_eq!(commands[0].value, "{\"text\":\"hello\"}");
}

#[tokio::test]
async fn send_panel_command_requires_payload() {
    let (_tmp, state) = test_state();
    let run_id = "panel-run-empty";
    setup_panel_run(&state.home, run_id);

    let resp = handlers::send_command(
        State(state),
        Path(run_id.to_string()),
        Json(
            serde_json::from_value(serde_json::json!({
                "output_id": "input"
            }))
            .unwrap(),
        ),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(body.contains("Missing command payload"));
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
