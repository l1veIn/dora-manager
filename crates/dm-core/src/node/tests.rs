use std::path::{Path, PathBuf};

use crate::registry::NodeMeta;
use tempfile::tempdir;

use super::download::download_registry_node;
use super::*;

#[test]
fn test_list_nodes_empty() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let nodes = list_nodes(home).unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn test_node_status_not_installed() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let status = node_status(home, "nonexistent").unwrap();
    assert!(status.is_none());
}

#[test]
fn test_uninstall_nonexistent() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let result = uninstall_node(home, "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_install_and_list_and_uninstall() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    let id = "test-node";

    let node_path = node_dir(home, id);
    std::fs::create_dir_all(&node_path).unwrap();

    let meta = NodeMetaFile {
        id: id.to_string(),
        name: String::new(),
        version: "1.0.0".to_string(),
        installed_at: "1234567890".to_string(),
        source: NodeSource {
            build: "python".to_string(),
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

    let meta_json = serde_json::to_string_pretty(&meta).unwrap();
    std::fs::write(dm_json_path(home, id), meta_json).unwrap();

    let nodes = list_nodes(home).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, id);
    assert_eq!(nodes[0].version, "1.0.0");

    let status = node_status(home, id).unwrap().unwrap();
    assert_eq!(status.id, id);

    uninstall_node(home, id).unwrap();
    assert!(!node_path.exists());

    let nodes = list_nodes(home).unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn test_nodes_dir_path() {
    let home = Path::new("/home/user/.dm");
    assert_eq!(
        paths::nodes_dir(home),
        PathBuf::from("/home/user/.dm/nodes")
    );
}

#[test]
fn test_node_dir_path() {
    let home = Path::new("/home/user/.dm");
    assert_eq!(
        node_dir(home, "llama-vision"),
        PathBuf::from("/home/user/.dm/nodes/llama-vision")
    );
}

#[test]
fn test_dm_json_path() {
    let home = Path::new("/home/user/.dm");
    assert_eq!(
        dm_json_path(home, "test"),
        PathBuf::from("/home/user/.dm/nodes/test/dm.json")
    );
}

#[tokio::test]
async fn test_download_registry_node_writes_metadata_without_source_checkout() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    let meta = NodeMeta {
        id: "demo-node".to_string(),
        name: "Demo Node".to_string(),
        description: "test".to_string(),
        build: "pip install dora-demo".to_string(),
        system_deps: None,
        inputs: vec!["in".to_string()],
        outputs: vec!["out".to_string()],
        tags: vec!["demo".to_string()],
        category: "Test".to_string(),
        github: None,
        source: None,
    };

    let entry = download_registry_node(home, "demo-node", &meta)
        .await
        .unwrap();
    assert_eq!(entry.id, "demo-node");
    assert_eq!(entry.name, "Demo Node");

    let dm: NodeMetaFile =
        serde_json::from_str(&std::fs::read_to_string(dm_json_path(home, "demo-node")).unwrap())
            .unwrap();
    assert_eq!(dm.executable, "");
    assert_eq!(dm.source.build, "pip install dora-demo");
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
