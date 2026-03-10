use std::fs;

use tempfile::tempdir;

use crate::dataflow::transpile_graph;
use crate::node::{node_dir, Node, NodeDisplay, NodeFiles, NodeRuntime, NodeSource};

fn setup_managed_node(home: &std::path::Path, id: &str, executable: &str) {
    let dir = node_dir(home, id);
    fs::create_dir_all(&dir).unwrap();

    // Create the executable file so the path resolves
    let exec_path = dir.join(executable);
    fs::create_dir_all(exec_path.parent().unwrap()).unwrap();
    fs::write(&exec_path, "#!/bin/bash\n# stub").unwrap();

    let meta = Node {
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
        path: Default::default(),
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
    node: test-node
"#,
    )
    .unwrap();

    let out = transpile_graph(home, &yaml_path).unwrap().yaml;
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
    // `node:` should be removed, `path:` should be the resolved absolute exec
    assert!(
        node.get(serde_yaml::Value::String("node".into())).is_none(),
        "node: field should be removed after transpile"
    );
    assert!(node
        .get(serde_yaml::Value::String("custom".into()))
        .is_none());
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
    node: unknown-node
"#,
    )
    .unwrap();

    let out = transpile_graph(home, &yaml_path).unwrap().yaml;
    // Unknown node: `node:` stays as-is (no path resolution)
    assert_eq!(out["nodes"][0]["node"].as_str(), Some("unknown-node"));
    assert!(out["nodes"][0]["path"].is_null());
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
    assert_eq!(list2[0].file.name, "my_flow");
    assert_eq!(list2[0].file.filename, "my_flow/dataflow.yml");
    assert_eq!(list2[0].meta.name, "my_flow");

    // 4. Get the content
    let project = crate::dataflow::get(home, "my_flow").unwrap();
    assert_eq!(project.yaml, yaml_content);
    assert_eq!(project.name, "my_flow");

    assert!(home.join("dataflows/my_flow/dataflow.yml").exists());
    assert!(home.join("dataflows/my_flow/flow.json").exists());
    assert!(home.join("dataflows/my_flow/config.json").exists());

    // 5. Delete it
    crate::dataflow::delete(home, "my_flow").unwrap();

    // 6. List should be empty again
    let list3 = crate::dataflow::list(home).unwrap();
    assert!(list3.is_empty());

    // 7. Get should fail
    let err = crate::dataflow::get(home, "my_flow")
        .unwrap_err()
        .to_string();
    assert!(err.contains("Failed to read dataflow"));
}

#[test]
fn test_dataflow_save_creates_history_snapshot() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();

    crate::dataflow::save(home, "history_flow", "nodes: []\n").unwrap();
    crate::dataflow::save(home, "history_flow", "nodes:\n  - id: a\n").unwrap();

    let history_dir = home.join("dataflows/history_flow/.history");
    let entries: Vec<_> = std::fs::read_dir(history_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1);
    let snapshot = std::fs::read_to_string(&entries[0]).unwrap();
    assert_eq!(snapshot, "nodes: []\n");
}

#[test]
fn test_migrate_legacy_dataflow_layout() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    std::fs::create_dir_all(home.join("dataflows")).unwrap();
    std::fs::write(home.join("dataflows/demo.yml"), "nodes: []\n").unwrap();

    let migrated = crate::dataflow::migrate_legacy_layout(home).unwrap();
    assert_eq!(migrated, 1);
    assert!(!home.join("dataflows/demo.yml").exists());
    assert_eq!(
        std::fs::read_to_string(home.join("dataflows/demo/dataflow.yml")).unwrap(),
        "nodes: []\n"
    );
    assert!(home.join("dataflows/demo/flow.json").exists());
    assert!(home.join("dataflows/demo/config.json").exists());
}

#[test]
fn test_import_dataflow_from_local_yaml_file() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    let source = home.join("source.yml");
    std::fs::write(&source, "nodes: []\n").unwrap();

    crate::dataflow::import_local(home, "imported-flow", &source).unwrap();

    assert_eq!(
        std::fs::read_to_string(home.join("dataflows/imported-flow/dataflow.yml")).unwrap(),
        "nodes: []\n"
    );
    assert!(home.join("dataflows/imported-flow/flow.json").exists());
    assert!(home.join("dataflows/imported-flow/config.json").exists());
}

#[test]
fn test_import_dataflow_from_local_directory() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    let source_dir = home.join("source-flow");
    std::fs::create_dir_all(&source_dir).unwrap();
    std::fs::write(source_dir.join("dataflow.yml"), "nodes:\n  - id: a\n").unwrap();
    std::fs::write(source_dir.join("config.json"), "{\n  \"a\": 1\n}\n").unwrap();

    crate::dataflow::import_local(home, "copied-flow", &source_dir).unwrap();

    assert_eq!(
        std::fs::read_to_string(home.join("dataflows/copied-flow/dataflow.yml")).unwrap(),
        "nodes:\n  - id: a\n"
    );
    assert_eq!(
        std::fs::read_to_string(home.join("dataflows/copied-flow/config.json")).unwrap(),
        "{\n  \"a\": 1\n}\n"
    );
}

#[test]
fn test_infer_import_name_from_github_blob_url() {
    let name = crate::dataflow::infer_import_name(
        "https://github.com/l1veIn/dora-manager/blob/master/tests/dataflows/system-test-full.yml",
    );
    assert_eq!(name, "system-test-full");
}

#[test]
fn test_inspect_config_aggregates_schema_and_effective_values() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();

    let node_dir = crate::node::node_dir(home, "cfg-node");
    std::fs::create_dir_all(&node_dir).unwrap();
    let meta = crate::node::Node {
        id: "cfg-node".to_string(),
        name: "cfg-node".to_string(),
        version: "1.0.0".to_string(),
        installed_at: "123".to_string(),
        source: crate::node::NodeSource {
            build: "pip install -e .".to_string(),
            github: None,
        },
        description: String::new(),
        executable: String::new(),
        repository: None,
        maintainers: Vec::new(),
        license: None,
        display: crate::node::NodeDisplay::default(),
        capabilities: Vec::new(),
        runtime: crate::node::NodeRuntime::default(),
        ports: Vec::new(),
        files: crate::node::NodeFiles::default(),
        examples: Vec::new(),
        config_schema: Some(serde_json::json!({
            "mode": {
                "default": "default-mode",
                "x-widget": {
                    "type": "select",
                    "options": ["default-mode", "node-mode", "flow-mode", "inline-mode"]
                }
            },
            "threshold": {
                "default": 0.2
            }
        })),
        path: Default::default(),
    };
    std::fs::write(
        node_dir.join("dm.json"),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();
    std::fs::write(
        node_dir.join("config.json"),
        serde_json::json!({
            "mode": "node-mode",
            "threshold": 0.5
        })
        .to_string(),
    )
    .unwrap();

    crate::dataflow::save(
        home,
        "cfg-flow",
        r#"
nodes:
  - id: first
    node: cfg-node
    config:
      mode: inline-mode
  - id: second
    node: cfg-node
"#,
    )
    .unwrap();
    crate::dataflow::save_flow_config(
        home,
        "cfg-flow",
        &serde_json::json!({
            "second": {
                "mode": "flow-mode"
            }
        }),
    )
    .unwrap();

    let doc = crate::dataflow::inspect_config(home, "cfg-flow").unwrap();
    assert_eq!(doc.nodes.len(), 2);
    let first = &doc.nodes[0];
    assert_eq!(first.yaml_id, "first");
    assert_eq!(first.fields["mode"].effective_source, "inline");
    assert_eq!(
        first.fields["mode"].effective_value,
        Some(serde_json::json!("inline-mode"))
    );
    assert_eq!(first.fields["threshold"].effective_source, "node");

    let second = &doc.nodes[1];
    assert_eq!(second.yaml_id, "second");
    assert_eq!(second.fields["mode"].effective_source, "flow");
    assert_eq!(
        second.fields["mode"].effective_value,
        Some(serde_json::json!("flow-mode"))
    );
    assert_eq!(
        second.fields["threshold"].effective_value,
        Some(serde_json::json!(0.5))
    );
}
