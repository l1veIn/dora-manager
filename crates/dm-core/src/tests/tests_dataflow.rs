use std::fs;

use tempfile::tempdir;

use crate::dataflow::transpile_graph;
use crate::node::{NodeMetaFile, NodeSource, node_dir};

fn setup_managed_python_node(home: &std::path::Path, id: &str) {
    let dir = node_dir(home, id);
    fs::create_dir_all(dir.join(".venv/bin")).unwrap();
    fs::create_dir_all(dir.join(".venv/lib/python3.12/site-packages")).unwrap();

    let meta = NodeMetaFile {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        installed_at: "1234567890".to_string(),
        source: NodeSource {
            build: "pip install dora-test-node".to_string(),
            github: None,
        },
    };

    fs::write(
        dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();
}

#[test]
fn transpile_graph_rewrites_managed_node_path_to_custom_and_env() {
    let tmp = tempdir().unwrap();
    let home = tmp.path();
    setup_managed_python_node(home, "test-node");

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

    assert!(node.get(serde_yaml::Value::String("path".into())).is_none());

    let custom = node
        .get(serde_yaml::Value::String("custom".into()))
        .unwrap()
        .as_mapping()
        .unwrap();
    assert_eq!(
        custom
            .get(serde_yaml::Value::String("source".into()))
            .unwrap()
            .as_str(),
        Some("Local")
    );
    assert!(
        custom
            .get(serde_yaml::Value::String("path".into()))
            .unwrap()
            .as_str()
            .unwrap()
            .contains(".venv/bin/python")
    );
    assert_eq!(
        custom
            .get(serde_yaml::Value::String("args".into()))
            .unwrap()
            .as_str(),
        Some("-m test_node.main")
    );

    let env = node
        .get(serde_yaml::Value::String("env".into()))
        .unwrap()
        .as_mapping()
        .unwrap();
    assert!(
        env.get(serde_yaml::Value::String("PATH".into()))
            .unwrap()
            .as_str()
            .unwrap()
            .contains(".venv/bin")
    );
    assert!(env
        .get(serde_yaml::Value::String("PYTHONPATH".into()))
        .is_some());
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
