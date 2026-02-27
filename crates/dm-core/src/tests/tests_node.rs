//! Tests for the node module

use crate::node::{list_nodes, node_status, uninstall_node, NodeEntry};
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

    // Don't create nodes directory
    let nodes = list_nodes(home).unwrap();
    assert!(nodes.is_empty(), "Missing nodes directory should return empty list");
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
    // Test that NodeEntry can be created and serialized
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
    
    // Create a node directory manually
    let nodes_dir = home.join("nodes");
    let node_dir = nodes_dir.join("manual-node");
    std::fs::create_dir_all(&node_dir).unwrap();
    
    // Without metadata, list_nodes should still work
    let nodes = list_nodes(home).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, "manual-node");
    assert_eq!(nodes[0].version, "unknown");
}

#[test]
fn test_uninstall_removes_directory() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    
    // Create a node directory
    let nodes_dir = home.join("nodes");
    let node_dir = nodes_dir.join("to-remove");
    std::fs::create_dir_all(&node_dir).unwrap();
    
    assert!(node_dir.exists());
    
    // Uninstall
    uninstall_node(home, "to-remove").unwrap();
    
    assert!(!node_dir.exists(), "Node directory should be removed");
}
