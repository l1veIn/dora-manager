//! Tests for the node module

use crate::node::{
    create_node, dm_json_path, download_node, get_node_config, get_node_readme, install_node,
    list_nodes, node_dir, node_status, save_node_config, uninstall_node, NodeEntry, NodeMetaFile,
    NodeSource,
};
use tempfile::tempdir;

#[test]
fn test_list_nodes_empty_directory() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let nodes = list_nodes(home).unwrap();
    assert!(nodes.is_empty(), "Empty directory should return empty list");
}

#[test]
fn test_list_nodes_no_nodes_dir() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let nodes = list_nodes(home).unwrap();
    assert!(
        nodes.is_empty(),
        "Missing nodes directory should return empty list"
    );
}

#[test]
fn test_node_status_not_found() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let status = node_status(home, "nonexistent-node").unwrap();
    assert!(status.is_none(), "Nonexistent node should return None");
}

#[test]
fn test_uninstall_nonexistent_node() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let result = uninstall_node(home, "nonexistent-node");
    assert!(result.is_err(), "Uninstalling nonexistent node should fail");
}

#[test]
fn test_node_entry_struct() {
    let entry = NodeEntry {
        id: "test-node".to_string(),
        name: String::new(),
        version: "1.0.0".to_string(),
        path: std::path::PathBuf::from("/test/path"),
        installed_at: "1234567890".to_string(),
        description: String::new(),
        author: None,
        category: String::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        avatar: None,
    };

    assert_eq!(entry.id, "test-node");
    assert_eq!(entry.version, "1.0.0");
}

#[test]
fn test_list_nodes_with_manual_entry() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let manual_dir = node_dir(home, "manual-node");
    std::fs::create_dir_all(&manual_dir).unwrap();

    let nodes = list_nodes(home).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, "manual-node");
    assert_eq!(nodes[0].version, "unknown");
}

#[test]
fn test_uninstall_removes_directory() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let installed_dir = node_dir(home, "to-remove");
    std::fs::create_dir_all(&installed_dir).unwrap();
    assert!(installed_dir.exists());

    uninstall_node(home, "to-remove").unwrap();
    assert!(!installed_dir.exists(), "Node directory should be removed");
}

#[test]
fn test_create_node_generates_scaffold() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let entry = create_node(home, "my-processor", "A test processor").unwrap();
    assert_eq!(entry.id, "my-processor");
    assert_eq!(entry.version, "0.1.0");

    let node_path = node_dir(home, "my-processor");
    assert!(node_path.join("dm.json").exists());
    assert!(node_path.join("pyproject.toml").exists());
    assert!(node_path.join("README.md").exists());
    assert!(node_path.join("my_processor/main.py").exists());
    assert!(node_path.join("my_processor/__init__.py").exists());

    let dm: NodeMetaFile =
        serde_json::from_str(&std::fs::read_to_string(node_path.join("dm.json")).unwrap()).unwrap();
    assert_eq!(dm.executable, "");
    assert_eq!(dm.id, "my-processor");

    let pyproject = std::fs::read_to_string(node_path.join("pyproject.toml")).unwrap();
    assert!(pyproject.contains("my-processor = \"my_processor.main:main\""));
    assert!(pyproject.contains("dora-rs"));

    let err = create_node(home, "my-processor", "").unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn test_config_crud() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    create_node(home, "test-cfg", "test").unwrap();

    let config = get_node_config(home, "test-cfg").unwrap();
    assert_eq!(config, serde_json::json!({}));

    let new_config = serde_json::json!({ "threshold": 0.8, "api_key": "sk-123" });
    save_node_config(home, "test-cfg", &new_config).unwrap();

    let config = get_node_config(home, "test-cfg").unwrap();
    assert_eq!(config["threshold"], 0.8);
    assert_eq!(config["api_key"], "sk-123");
}

#[test]
fn test_get_node_readme_returns_local_content() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    create_node(home, "readme-node", "Readable").unwrap();
    let readme = get_node_readme(home, "readme-node").unwrap();
    assert!(readme.contains("# readme-node"));
}

#[test]
fn test_save_node_config_requires_existing_node() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let err = save_node_config(home, "missing-node", &serde_json::json!({ "a": 1 })).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_download_node_fails_fast_when_directory_exists() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    std::fs::create_dir_all(node_dir(home, "demo-node")).unwrap();

    let err = download_node(home, "demo-node").await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_install_node_errors_when_missing() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let err = install_node(home, "missing-node").await.unwrap_err();
    assert!(err.to_string().contains("Download or create it first"));
}

#[tokio::test]
async fn test_install_node_errors_for_unsupported_build() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    let id = "bad-build";
    let node_path = node_dir(home, id);
    std::fs::create_dir_all(&node_path).unwrap();

    let meta = NodeMetaFile {
        id: id.to_string(),
        name: id.to_string(),
        version: String::new(),
        installed_at: "1234567890".to_string(),
        source: NodeSource {
            build: "npm install bad-build".to_string(),
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
        dm_json_path(home, id),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();

    let err = install_node(home, id).await.unwrap_err();
    assert!(err.to_string().contains("Unsupported build type"));
}
