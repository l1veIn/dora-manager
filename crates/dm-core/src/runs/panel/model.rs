use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub seq: i64,
    pub input_id: String,
    pub producer_id: Option<String>,
    pub output_field: Option<String>,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub storage: String,
    pub path: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AssetFilter {
    pub since_seq: Option<i64>,
    pub before_seq: Option<i64>,
    pub input_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedAssets {
    pub assets: Vec<Asset>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputCommand {
    pub seq: i64,
    pub output_id: String,
    pub value: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelRun {
    pub run_id: String,
    pub asset_count: i64,
    pub command_count: i64,
    pub disk_size_bytes: u64,
    pub last_modified: String,
}
