use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::registry::NodeMeta;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataflowGraph {
    pub nodes: Vec<NodeInstance>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeInstance {
    pub id: String,
    pub node_type: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Edge {
    pub from_node: String,
    pub from_output: String,
    pub to_node: String,
    pub to_input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl DataflowGraph {
    pub fn from_yaml_str(content: &str) -> serde_yaml::Result<Self> {
        serde_yaml::from_str(content)
    }

    pub fn from_json_str(content: &str) -> serde_json::Result<Self> {
        serde_json::from_str(content)
    }

    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self::from_yaml_str(&content)?)
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self::from_json_str(&content)?)
    }
}

pub fn validate_graph(graph: &DataflowGraph, registry: &[NodeMeta]) -> ValidationResult {
    let mut errors = Vec::new();
    let warnings = Vec::new();

    let registry_map: HashMap<&str, &NodeMeta> = registry.iter().map(|m| (m.id.as_str(), m)).collect();

    let mut seen_ids = HashSet::new();
    for node in &graph.nodes {
        if !seen_ids.insert(node.id.as_str()) {
            errors.push(format!("duplicate node id: {}", node.id));
        }

        if !registry_map.contains_key(node.node_type.as_str()) {
            errors.push(format!(
                "unknown node type '{}' for node '{}'",
                node.node_type, node.id
            ));
        }
    }

    let node_map: HashMap<&str, &NodeInstance> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for edge in &graph.edges {
        let from_node = node_map.get(edge.from_node.as_str());
        let to_node = node_map.get(edge.to_node.as_str());

        if from_node.is_none() {
            errors.push(format!("edge references missing from_node '{}'", edge.from_node));
            continue;
        }
        if to_node.is_none() {
            errors.push(format!("edge references missing to_node '{}'", edge.to_node));
            continue;
        }

        let from_node = *from_node.expect("checked");
        let to_node = *to_node.expect("checked");

        if let Some(meta) = registry_map.get(from_node.node_type.as_str()) {
            if !meta.outputs.iter().any(|o| o == &edge.from_output) {
                errors.push(format!(
                    "invalid output port '{}' on node '{}' (type '{}')",
                    edge.from_output, from_node.id, from_node.node_type
                ));
            }
        }

        if let Some(meta) = registry_map.get(to_node.node_type.as_str()) {
            if !meta.inputs.iter().any(|i| i == &edge.to_input) {
                errors.push(format!(
                    "invalid input port '{}' on node '{}' (type '{}')",
                    edge.to_input, to_node.id, to_node.node_type
                ));
            }
        }
    }

    let cycles = detect_cycles(graph);
    for cycle in cycles {
        errors.push(format!("cycle detected: {}", cycle.join(" -> ")));
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

pub fn detect_cycles(graph: &DataflowGraph) -> Vec<Vec<String>> {
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for node in &graph.nodes {
        adjacency.entry(node.id.clone()).or_default();
    }

    for edge in &graph.edges {
        if node_ids.contains(edge.from_node.as_str()) && node_ids.contains(edge.to_node.as_str()) {
            adjacency
                .entry(edge.from_node.clone())
                .or_default()
                .push(edge.to_node.clone());
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    fn dfs(
        node: &str,
        adjacency: &HashMap<String, Vec<String>>,
        colors: &mut HashMap<String, Color>,
        stack: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        colors.insert(node.to_string(), Color::Gray);
        stack.push(node.to_string());

        if let Some(neighbors) = adjacency.get(node) {
            for next in neighbors {
                let state = colors.get(next.as_str()).copied().unwrap_or(Color::White);
                match state {
                    Color::White => dfs(next, adjacency, colors, stack, cycles),
                    Color::Gray => {
                        if let Some(pos) = stack.iter().position(|id| id == next) {
                            let mut cycle = stack[pos..].to_vec();
                            cycle.push(next.clone());
                            cycles.push(cycle);
                        }
                    }
                    Color::Black => {}
                }
            }
        }

        stack.pop();
        colors.insert(node.to_string(), Color::Black);
    }

    let mut colors: HashMap<String, Color> = adjacency
        .keys()
        .cloned()
        .map(|id| (id, Color::White))
        .collect();
    let mut stack = Vec::new();
    let mut cycles = Vec::new();

    let keys: Vec<String> = adjacency.keys().cloned().collect();
    for node in keys {
        if colors.get(node.as_str()).copied().unwrap_or(Color::White) == Color::White {
            dfs(&node, &adjacency, &mut colors, &mut stack, &mut cycles);
        }
    }

    cycles
}
