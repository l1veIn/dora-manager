use std::fs;

use tempfile::tempdir;

use crate::dataflow::transpile_graph;
use crate::node::{NodeMetaFile, NodeSource, node_dir};

fn setup_managed_node(home: &std::path::Path, id: &str, executable: &str) {
    let dir = node_dir(home, id);
    fs::create_dir_all(&dir).unwrap();

    // Create the executable file so the path resolves
    let exec_path = dir.join(executable);
    fs::create_dir_all(exec_path.parent().unwrap()).unwrap();
    fs::write(&exec_path, "#!/bin/bash\n# stub").unwrap();

    let meta = NodeMetaFile {
        id: id.to_string(),
        name: String::new(),
        version: "1.0.0".to_string(),
        installed_at: "1234567890".to_string(),
        source: NodeSource {
            build: "pip install dora-test-node".to_string(),
            github: None,
        },
        description: String::new(),
        executable: executable.to_string(),
        author: None,
        category: String::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        avatar: None,
        config_schema: None,
    };

    fs::write(
        dir.join("dm.json"),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();
}

#[test]
fn transpile_graph_resolves_executable_path() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    setup_managed_node(home, "test-node", ".venv/bin/test-node");

    let yaml_path = home.join("graph.yml");
    fs::write(
        &yaml_path,
        r#"
nodes:
  - id: n1
    path: test-node
"#,
    )
    .unwrap();

    let out = transpile_graph(home, &yaml_path).unwrap();
    let nodes = out["nodes"].as_sequence().unwrap();
    let node = nodes[0].as_mapping().unwrap();

    // path should be resolved to absolute executable path
    let path_val = node
        .get(serde_yaml::Value::String("path".into()))
        .unwrap()
        .as_str()
        .unwrap();
    assert!(path_val.contains(".venv/bin/test-node"));
    assert!(path_val.starts_with("/"), "Path should be absolute");

    // No custom block, no env block — executable handles everything
    assert!(node.get(serde_yaml::Value::String("custom".into())).is_none());
    assert!(node.get(serde_yaml::Value::String("env".into())).is_none());
}

#[test]
fn transpile_graph_leaves_unknown_node_path_unchanged() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    let yaml_path = home.join("graph.yml");

    fs::write(
        &yaml_path,
        r#"
nodes:
  - id: n1
    path: unknown-node
"#,
    )
    .unwrap();

    let out = transpile_graph(home, &yaml_path).unwrap();
    assert_eq!(out["nodes"][0]["path"].as_str(), Some("unknown-node"));
    assert!(out["nodes"][0]["custom"].is_null());
}

#[test]
fn transpile_graph_errors_on_invalid_yaml() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    let yaml_path = home.join("bad.yml");
    fs::write(&yaml_path, "nodes: [").unwrap();

    let err = transpile_graph(home, &yaml_path).unwrap_err().to_string();
    assert!(err.contains("Failed to parse yaml"));
}


#[test]
fn transpile_graph_errors_on_missing_file() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    let yaml_path = home.join("missing.yml");

    let err = transpile_graph(home, &yaml_path).unwrap_err().to_string();
    assert!(err.contains("Failed to read graph yaml"));
}

#[test]
fn test_dataflow_crud() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();

    // 1. Initial list should be empty
    let list1 = crate::dataflow::list(home).unwrap();
    assert!(list1.is_empty());

    // 2. Save a new dataflow
    let yaml_content = "nodes:\n  - id: test\n";
    crate::dataflow::save(home, "my_flow", yaml_content).unwrap();

    // 3. List should now contain 1 item
    let list2 = crate::dataflow::list(home).unwrap();
    assert_eq!(list2.len(), 1);
    assert_eq!(list2[0].name, "my_flow");
    assert_eq!(list2[0].filename, "my_flow.yml");

    // 4. Get the content
    let stored_yaml = crate::dataflow::get(home, "my_flow").unwrap();
    assert_eq!(stored_yaml, yaml_content);

    // 5. Delete it
    crate::dataflow::delete(home, "my_flow").unwrap();

    // 6. List should be empty again
    let list3 = crate::dataflow::list(home).unwrap();
    assert!(list3.is_empty());

    // 7. Get should fail
    let err = crate::dataflow::get(home, "my_flow").unwrap_err().to_string();
    assert!(err.contains("Failed to read dataflow"));
}
