//! Tests for the node module

use crate::node::{
    create_node, dm_json_path, get_node_config, get_node_readme, git_like_file_tree, install_node,
    list_nodes, node_dir, node_status, read_node_file, save_node_config, uninstall_node, Node,
    NodeDisplay, NodeFiles, NodeRuntime, NodeSource,
};
use tempfile::tempdir;

fn builtin_test_node_ids() -> [&'static str; 2] {
    ["dm-test-audio-capture", "dm-test-media-capture"]
}

#[test]
fn test_list_nodes_empty_directory() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let nodes = list_nodes(home).unwrap();
    for id in builtin_test_node_ids() {
        assert!(
            nodes.iter().any(|node| node.id == id),
            "Builtin node '{}' should be discoverable",
            id
        );
    }
}

#[test]
fn test_list_nodes_no_nodes_dir() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let nodes = list_nodes(home).unwrap();
    for id in builtin_test_node_ids() {
        assert!(nodes.iter().any(|node| node.id == id));
    }
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
fn test_node_struct() {
    let node = Node {
        id: "test-node".to_string(),
        name: String::new(),
        version: "1.0.0".to_string(),
        installed_at: "1234567890".to_string(),
        source: NodeSource {
            build: String::new(),
            github: None,
        },
        description: String::new(),
        executable: String::new(),
        repository: None,
        maintainers: Vec::new(),
        license: None,
        display: NodeDisplay::default(),
        dm: None,
        capabilities: Vec::new(),
        runtime: NodeRuntime::default(),
        ports: Vec::new(),
        files: NodeFiles::default(),
        examples: Vec::new(),
        config_schema: None,
        dynamic_ports: false,
        interaction: None,
        path: std::path::PathBuf::from("/test/path"),
    };

    assert_eq!(node.id, "test-node");
    assert_eq!(node.version, "1.0.0");
}

#[test]
fn test_list_nodes_with_manual_entry() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let manual_dir = node_dir(home, "manual-node");
    std::fs::create_dir_all(&manual_dir).unwrap();

    let nodes = list_nodes(home).unwrap();
    let manual = nodes.iter().find(|node| node.id == "manual-node").unwrap();
    assert_eq!(manual.version, "unknown");
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
fn test_uninstall_builtin_node_rejected() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let err = uninstall_node(home, "dm-test-media-capture").unwrap_err();
    assert!(err.to_string().contains("builtin"));
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

    let dm: Node =
        serde_json::from_str(&std::fs::read_to_string(node_path.join("dm.json")).unwrap()).unwrap();
    assert_eq!(dm.executable, "");
    assert_eq!(dm.files.readme, "README.md");
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
fn test_git_like_file_tree_lists_relative_files_and_skips_cache_dirs() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    create_node(home, "tree-node", "Tree").unwrap();
    let node_path = node_dir(home, "tree-node");
    std::fs::create_dir_all(node_path.join("nested")).unwrap();
    std::fs::create_dir_all(node_path.join("__pycache__")).unwrap();
    std::fs::create_dir_all(node_path.join("node_modules/pkg")).unwrap();
    std::fs::create_dir_all(node_path.join("target/debug")).unwrap();
    std::fs::write(node_path.join("nested/config.yaml"), "name: tree-node\n").unwrap();
    std::fs::write(node_path.join("__pycache__/ignored.pyc"), "compiled").unwrap();
    std::fs::write(node_path.join("node_modules/pkg/index.js"), "ignored").unwrap();
    std::fs::write(node_path.join("target/debug/app"), "ignored").unwrap();

    let files = git_like_file_tree(home, "tree-node").unwrap();
    assert!(files.contains(&"README.md".to_string()));
    assert!(files.contains(&"pyproject.toml".to_string()));
    assert!(files.contains(&"nested/config.yaml".to_string()));
    assert!(!files.iter().any(|path| path.contains("__pycache__")));
    assert!(!files.iter().any(|path| path.contains("node_modules")));
    assert!(!files.iter().any(|path| path.contains("target/")));
}

#[test]
fn test_read_node_file_reads_text_content() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    create_node(home, "file-node", "Files").unwrap();
    let content = read_node_file(home, "file-node", "pyproject.toml").unwrap();
    assert!(content.contains("[project]"));
}

#[test]
fn test_read_node_file_rejects_directory_traversal() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    create_node(home, "secure-node", "Secure").unwrap();
    let err = read_node_file(home, "secure-node", "../outside.txt").unwrap_err();
    assert!(err.to_string().contains("Invalid node file path"));
}

#[test]
fn test_save_node_config_requires_existing_node() {
    let dir = tempdir().unwrap();
    let home = dir.path();

    let err = save_node_config(home, "missing-node", &serde_json::json!({ "a": 1 })).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
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
        dm: None,
        capabilities: Vec::new(),
        runtime: NodeRuntime::default(),
        ports: Vec::new(),
        files: NodeFiles::default(),
        examples: Vec::new(),
        config_schema: None,
        dynamic_ports: false,
        interaction: None,
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
