use std::collections::HashMap;

use serde_json::json;
use tempfile::TempDir;

use crate::graph::{detect_cycles, validate_graph, DataflowGraph, Edge, NodeInstance};
use crate::registry::NodeMeta;

fn node_meta(id: &str, inputs: &[&str], outputs: &[&str]) -> NodeMeta {
    NodeMeta {
        id: id.to_string(),
        name: id.to_string(),
        description: "test".to_string(),
        build: "noop".to_string(),
        system_deps: None,
        inputs: inputs.iter().map(|s| s.to_string()).collect(),
        outputs: outputs.iter().map(|s| s.to_string()).collect(),
        tags: vec!["test".to_string()],
        category: "Test".to_string(),
    }
}

#[test]
fn validate_valid_graph() {
    let registry = vec![
        node_meta("source", &[], &["out"]),
        node_meta("sink", &["in"], &[]),
    ];

    let graph = DataflowGraph {
        nodes: vec![
            NodeInstance {
                id: "n1".to_string(),
                node_type: "source".to_string(),
                config: HashMap::from([("rate".to_string(), json!(30))]),
            },
            NodeInstance {
                id: "n2".to_string(),
                node_type: "sink".to_string(),
                config: HashMap::new(),
            },
        ],
        edges: vec![Edge {
            from_node: "n1".to_string(),
            from_output: "out".to_string(),
            to_node: "n2".to_string(),
            to_input: "in".to_string(),
        }],
    };

    let result = validate_graph(&graph, &registry);
    assert!(result.valid, "errors: {:?}", result.errors);
    assert!(result.errors.is_empty());
}

#[test]
fn detect_cycle_in_graph() {
    let graph = DataflowGraph {
        nodes: vec![
            NodeInstance {
                id: "a".to_string(),
                node_type: "t".to_string(),
                config: HashMap::new(),
            },
            NodeInstance {
                id: "b".to_string(),
                node_type: "t".to_string(),
                config: HashMap::new(),
            },
        ],
        edges: vec![
            Edge {
                from_node: "a".to_string(),
                from_output: "o".to_string(),
                to_node: "b".to_string(),
                to_input: "i".to_string(),
            },
            Edge {
                from_node: "b".to_string(),
                from_output: "o".to_string(),
                to_node: "a".to_string(),
                to_input: "i".to_string(),
            },
        ],
    };

    let cycles = detect_cycles(&graph);
    assert!(!cycles.is_empty());
    assert!(cycles.iter().any(|c| c.first() == Some(&"a".to_string()) || c.first() == Some(&"b".to_string())));
}

#[test]
fn validate_invalid_port_reports_error() {
    let registry = vec![
        node_meta("producer", &[], &["out"]),
        node_meta("consumer", &["in"], &[]),
    ];

    let graph = DataflowGraph {
        nodes: vec![
            NodeInstance {
                id: "p".to_string(),
                node_type: "producer".to_string(),
                config: HashMap::new(),
            },
            NodeInstance {
                id: "c".to_string(),
                node_type: "consumer".to_string(),
                config: HashMap::new(),
            },
        ],
        edges: vec![Edge {
            from_node: "p".to_string(),
            from_output: "wrong_out".to_string(),
            to_node: "c".to_string(),
            to_input: "in".to_string(),
        }],
    };

    let result = validate_graph(&graph, &registry);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("invalid output port")));
}

#[test]
fn parse_graph_from_yaml_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("graph.yaml");
    let yaml = r#"
nodes:
  - id: source_1
    node_type: source
    config:
      enabled: true
      threshold: 0.5
  - id: sink_1
    node_type: sink
    config: {}
edges:
  - from_node: source_1
    from_output: out
    to_node: sink_1
    to_input: in
"#;
    std::fs::write(&path, yaml).unwrap();

    let graph = DataflowGraph::from_yaml_file(&path).unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.nodes[0].id, "source_1");
    assert_eq!(graph.edges[0].to_input, "in");
}
