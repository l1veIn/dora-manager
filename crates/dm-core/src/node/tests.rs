use std::path::{Path, PathBuf};

use tempfile::tempdir;

use super::*;

fn builtin_test_node_ids() -> [&'static str; 4] {
    [
        "dm-mjpeg",
        "dm-queue",
        "dm-test-audio-capture",
        "dm-test-media-capture",
    ]
}

#[test]
fn test_list_nodes_empty() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let nodes = list_nodes(home).unwrap();
    for id in builtin_test_node_ids() {
        assert!(nodes.iter().any(|node| node.id == id));
    }
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

    let node = Node {
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
        repository: None,
        maintainers: Vec::new(),
        license: None,
        display: NodeDisplay::default(),
        capabilities: Vec::new(),
        runtime: NodeRuntime::default(),
        ports: Vec::new(),
        files: NodeFiles::default(),
        examples: Vec::new(),
        config_schema: None,
        dynamic_ports: false,
        path: Default::default(),
    };

    let meta_json = serde_json::to_string_pretty(&node).unwrap();
    std::fs::write(dm_json_path(home, id), meta_json).unwrap();

    let nodes = list_nodes(home).unwrap();
    let installed = nodes.iter().find(|node| node.id == id).unwrap();
    assert_eq!(installed.version, "1.0.0");

    let status = node_status(home, id).unwrap().unwrap();
    assert_eq!(status.id, id);

    uninstall_node(home, id).unwrap();
    assert!(!node_path.exists());

    let nodes = list_nodes(home).unwrap();
    assert!(!nodes.iter().any(|node| node.id == id));
}

#[test]
fn test_builtin_node_cannot_be_uninstalled() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let err = uninstall_node(home, "dm-test-media-capture").unwrap_err();
    assert!(err.to_string().contains("builtin"));
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

    let node = Node {
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
        repository: None,
        maintainers: Vec::new(),
        license: None,
        display: NodeDisplay::default(),
        capabilities: Vec::new(),
        runtime: NodeRuntime::default(),
        ports: Vec::new(),
        files: NodeFiles::default(),
        examples: Vec::new(),
        config_schema: None,
        dynamic_ports: false,
        path: Default::default(),
    };
    std::fs::write(
        dm_json_path(home, id),
        serde_json::to_string_pretty(&node).unwrap(),
    )
    .unwrap();

    let err = install_node(home, id).await.unwrap_err();
    assert!(err.to_string().contains("Unsupported build type"));
}
