use std::sync::Arc;

use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use tempfile::TempDir;

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
    details: String,
}

fn sample_registry_node() -> dm_core::registry::NodeMeta {
    dm_core::registry::NodeMeta {
        id: "image-io".to_string(),
        name: "Image IO".to_string(),
        description: "Test node".to_string(),
        build: "python".to_string(),
        system_deps: None,
        inputs: vec!["image".to_string()],
        outputs: vec!["image".to_string()],
        tags: vec!["io".to_string()],
        category: "test".to_string(),
    }
}

async fn spawn_server(
    registry: Vec<dm_core::registry::NodeMeta>,
) -> (String, tokio::task::JoinHandle<()>, TempDir) {
    let tmp = TempDir::new().unwrap();
    let state = dm_core::api::AppState::new(Arc::new(tmp.path().to_path_buf()))
        .with_registry_override(registry);
    let app = dm_core::api::create_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), handle, tmp)
}

#[tokio::test]
async fn test_get_registry() {
    let (base_url, handle, _tmp) = spawn_server(vec![sample_registry_node()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/v1/registry", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
    assert_eq!(body[0]["id"], "image-io");

    handle.abort();
}

#[tokio::test]
async fn test_list_nodes_empty() {
    let (base_url, handle, _tmp) = spawn_server(vec![]).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/v1/nodes", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body, json!([]));

    handle.abort();
}

#[tokio::test]
async fn test_install_node_missing() {
    let (base_url, handle, _tmp) = spawn_server(vec![sample_registry_node()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/v1/nodes/install", base_url))
        .json(&json!({ "id": "missing-node" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let err: ErrorResponse = resp.json().await.unwrap();
    assert_eq!(err.error, "Failed to install node");
    assert!(err.details.contains("missing-node"));

    handle.abort();
}

#[tokio::test]
async fn test_uninstall_node() {
    let (base_url, handle, tmp) = spawn_server(vec![]).await;
    let client = reqwest::Client::new();

    let node_dir = tmp.path().join("nodes").join("demo-node");
    std::fs::create_dir_all(&node_dir).unwrap();
    std::fs::write(node_dir.join("meta.json"), "{}\n").unwrap();

    let resp = client
        .delete(format!("{}/api/v1/nodes/demo-node", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["message"], "Uninstalled node 'demo-node'");
    assert!(!node_dir.exists());

    handle.abort();
}

#[tokio::test]
async fn test_validate_graph() {
    let (base_url, handle, _tmp) = spawn_server(vec![sample_registry_node()]).await;
    let client = reqwest::Client::new();

    let yaml = r#"
nodes:
  - id: n1
    node_type: image-io
edges: []
"#;

    let resp = client
        .post(format!("{}/api/v1/graph/validate", base_url))
        .json(&json!({ "yaml": yaml }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["valid"], true);
    assert_eq!(body["errors"], json!([]));

    handle.abort();
}

#[tokio::test]
async fn test_run_graph_placeholder() {
    let (base_url, handle, _tmp) = spawn_server(vec![]).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/v1/graph/run", base_url))
        .json(&json!({ "yaml": "nodes: []\nedges: []\n" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["message"], "Not implemented yet");

    handle.abort();
}
