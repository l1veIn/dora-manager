use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;

use crate::node::Node;

use super::model::ManagedNode;

pub(crate) const HIDDEN_DM_BRIDGE_YAML_ID: &str = "__dm_bridge";
pub(crate) const DM_BRIDGE_INPUT_ENV_KEY: &str = "DM_BRIDGE_INPUT_PORT";
pub(crate) const DM_BRIDGE_OUTPUT_ENV_KEY: &str = "DM_BRIDGE_OUTPUT_PORT";
pub(crate) const DM_CAPABILITIES_ENV_KEY: &str = "DM_CAPABILITIES_JSON";
pub(crate) const NODE_DM_BRIDGE_INPUT_PORT: &str = "dm_bridge_input_internal";
pub(crate) const NODE_DM_BRIDGE_OUTPUT_PORT: &str = "dm_bridge_output_internal";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HiddenBridgeNodeSpec {
    pub yaml_id: String,
    pub node_id: String,
    pub env: BTreeMap<String, String>,
    pub bindings: Vec<HiddenBridgeBindingSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_input_port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_output_port: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HiddenBridgeBindingSpec {
    pub family: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

pub(crate) fn build_bridge_node_spec(
    meta: &Node,
    managed: &ManagedNode,
) -> Option<HiddenBridgeNodeSpec> {
    let bindings = meta
        .capability_bindings()
        .into_iter()
        .filter_map(|(family, binding)| match family.as_str() {
            "widget_input" | "display" => Some(HiddenBridgeBindingSpec {
                family,
                role: binding.role.clone(),
                port: binding.port.clone(),
                channel: binding.channel.clone(),
                media: binding.media.clone(),
                lifecycle: binding.lifecycle.clone(),
                description: binding.description.clone(),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    if bindings.is_empty() {
        return None;
    }

    let has_widget_input = bindings
        .iter()
        .any(|binding| binding.family == "widget_input");
    let has_display = bindings.iter().any(|binding| binding.family == "display");

    Some(HiddenBridgeNodeSpec {
        yaml_id: managed.yaml_id.clone(),
        node_id: meta.id.clone(),
        env: mapping_to_string_map(&managed.merged_env),
        bindings,
        bridge_input_port: has_display.then(|| bridge_input_port_for(&managed.yaml_id)),
        bridge_output_port: has_widget_input.then(|| bridge_output_port_for(&managed.yaml_id)),
    })
}

pub(crate) fn bridge_output_port_for(yaml_id: &str) -> String {
    format!("dm_bridge_to_{}", sanitize_port_suffix(yaml_id))
}

pub(crate) fn bridge_input_port_for(yaml_id: &str) -> String {
    format!("dm_display_from_{}", sanitize_port_suffix(yaml_id))
}

pub(crate) fn bridge_specs_json(specs: &[HiddenBridgeNodeSpec]) -> Result<String> {
    serde_json::to_string(specs).map_err(Into::into)
}

pub(crate) fn mapping_to_string_map(mapping: &serde_yaml::Mapping) -> BTreeMap<String, String> {
    mapping
        .iter()
        .filter_map(|(key, value)| Some((key.as_str()?.to_string(), yaml_value_to_string(value))))
        .collect()
}

pub(crate) fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(raw) => raw.clone(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

pub(crate) fn ensure_output_port(managed: &mut ManagedNode, port_id: &str) {
    let key = serde_yaml::Value::String("outputs".to_string());
    let existing = managed
        .extra_fields
        .remove(&key)
        .and_then(|value| value.as_sequence().cloned())
        .unwrap_or_default();

    let mut outputs = existing
        .into_iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();

    if !outputs.iter().any(|existing| existing == port_id) {
        outputs.push(port_id.to_string());
    }

    managed.extra_fields.insert(
        key,
        serde_yaml::Value::Sequence(outputs.into_iter().map(serde_yaml::Value::String).collect()),
    );
}

pub(crate) fn ensure_input_mapping(managed: &mut ManagedNode, input_port: &str, source: &str) {
    let key = serde_yaml::Value::String("inputs".to_string());
    let mut inputs = managed
        .extra_fields
        .remove(&key)
        .and_then(|value| value.as_mapping().cloned())
        .unwrap_or_default();

    inputs.insert(
        serde_yaml::Value::String(input_port.to_string()),
        serde_yaml::Value::String(source.to_string()),
    );

    managed
        .extra_fields
        .insert(key, serde_yaml::Value::Mapping(inputs));
}

fn sanitize_port_suffix(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{
        Node, NodeCapability, NodeCapabilityBinding, NodeCapabilityDetail, NodeDisplay, NodeFiles,
        NodeRuntime, NodeSource,
    };

    fn test_node(capabilities: Vec<NodeCapability>) -> Node {
        Node {
            id: "demo".to_string(),
            name: String::new(),
            version: "1.0.0".to_string(),
            installed_at: "123".to_string(),
            source: NodeSource {
                build: "pip install -e .".to_string(),
                github: None,
            },
            description: String::new(),
            executable: ".venv/bin/demo".to_string(),
            repository: None,
            maintainers: Vec::new(),
            license: None,
            display: NodeDisplay::default(),
            capabilities,
            runtime: NodeRuntime::default(),
            ports: Vec::new(),
            files: NodeFiles::default(),
            examples: Vec::new(),
            config_schema: None,
            dynamic_ports: false,
            path: Default::default(),
        }
    }

    fn managed_node() -> ManagedNode {
        ManagedNode {
            yaml_id: "prompt".to_string(),
            node_id: "demo".to_string(),
            inline_config: serde_json::json!({}),
            resolved_path: Some("/tmp/demo".to_string()),
            merged_env: serde_yaml::from_str("LABEL: Prompt\nDEFAULT_VALUE: hi\n").unwrap(),
            extra_fields: serde_yaml::Mapping::new(),
        }
    }

    #[test]
    fn build_bridge_node_spec_collects_supported_capabilities() {
        let node = test_node(vec![NodeCapability::Detail(NodeCapabilityDetail {
            name: "widget_input".to_string(),
            bindings: vec![NodeCapabilityBinding {
                role: "widget".to_string(),
                port: Some("value".to_string()),
                channel: Some("input".to_string()),
                media: vec!["text".to_string()],
                lifecycle: vec!["run_scoped".to_string()],
                description: None,
            }],
        })]);

        let spec = build_bridge_node_spec(&node, &managed_node()).expect("expected bridge spec");
        assert_eq!(spec.yaml_id, "prompt");
        assert_eq!(
            spec.bridge_output_port.as_deref(),
            Some("dm_bridge_to_prompt")
        );
        assert_eq!(spec.env.get("LABEL").map(String::as_str), Some("Prompt"));
        assert_eq!(spec.bindings.len(), 1);
        assert_eq!(spec.bindings[0].family, "widget_input");
    }

    #[test]
    fn build_bridge_node_spec_skips_unrelated_capabilities() {
        let node = test_node(vec![NodeCapability::Tag("configurable".to_string())]);
        assert!(build_bridge_node_spec(&node, &managed_node()).is_none());
    }
}
