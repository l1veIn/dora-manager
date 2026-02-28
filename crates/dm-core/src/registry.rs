use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/dora-rs/dora-rs.github.io/main/src/data/nodes.json";

/// Registry 中节点的基本信息（从 `nodes.json` 解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub build: String,
    pub system_deps: Option<HashMap<String, String>>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub github: Option<String>,
    pub source: Option<String>,
}

/// 从 `nodes.json` 解析的原始数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNode {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub install: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: String,
    pub preview: Option<String>,
    pub website: Option<String>,
    pub github: Option<String>,
    pub author: Option<String>,
    pub downloads: Option<String>,
    pub source: Option<String>,
    pub last_commit: Option<String>,
    pub last_release: Option<String>,
    pub license: Option<String>,
    pub support: Option<String>,
}

/// 解析 `install` YAML 字段得到的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    pub id: String,
    pub build: String,
    pub path: String,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    #[serde(default)]
    pub outputs: Vec<String>,
}

/// 从 GitHub 获取并解析节点注册表
pub async fn fetch_registry() -> Result<Vec<NodeMeta>> {
    let client = Client::new();
    let resp = client
        .get(REGISTRY_URL)
        .header("User-Agent", "dm/0.1")
        .send()
        .await?;

    if !resp.status().is_success() {
        bail!("Failed to fetch registry: {}", resp.status());
    }

    let raw_nodes: Vec<RegistryNode> = resp.json().await?;
    let nodes = raw_nodes
        .into_iter()
        .filter_map(|node| parse_registry_node(node).ok())
        .collect();

    Ok(nodes)
}

fn parse_registry_node(node: RegistryNode) -> Result<NodeMeta> {
    let config = parse_install_yaml(&node.install)
        .with_context(|| format!("failed to parse install yaml for {}", node.title))?;

    Ok(NodeMeta {
        id: config.id,
        name: node.title,
        description: node.description,
        build: config.build,
        system_deps: None,
        inputs: config.inputs.into_keys().collect(),
        outputs: config.outputs,
        tags: node.tags,
        category: node.category,
        github: node.github,
        source: node.source,
    })
}

/// 解析节点 `install` YAML 字符串
///
/// 实际 registry 中该字段通常是一个仅包含单个元素的 YAML 数组，
/// 这里同时兼容数组和单对象两种格式。
pub(crate) fn parse_install_yaml(yaml: &str) -> Result<InstallConfig> {
    if let Ok(mut configs) = serde_yaml::from_str::<Vec<InstallConfig>>(yaml) {
        if let Some(config) = configs.drain(..).next() {
            return Ok(config);
        }
        bail!("install yaml is empty");
    }

    let config: InstallConfig =
        serde_yaml::from_str(yaml).context("invalid install yaml format")?;
    Ok(config)
}

/// 过滤节点（支持名称、描述、标签搜索）
pub fn filter_nodes<'a>(nodes: &'a [NodeMeta], query: &str) -> Vec<&'a NodeMeta> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return nodes.iter().collect();
    }

    nodes
        .iter()
        .filter(|n| {
            n.name.to_lowercase().contains(&query)
                || n.description.to_lowercase().contains(&query)
                || n.tags.iter().any(|t| t.to_lowercase().contains(&query))
        })
        .collect()
}

/// 根据 ID 查找节点
pub fn find_node<'a>(nodes: &'a [NodeMeta], id: &str) -> Option<&'a NodeMeta> {
    nodes.iter().find(|n| n.id == id)
}
