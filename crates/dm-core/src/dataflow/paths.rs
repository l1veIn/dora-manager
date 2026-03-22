use std::path::{Path, PathBuf};

pub const DATAFLOW_FILE: &str = "dataflow.yml";
pub const FLOW_META_FILE: &str = "flow.json";
pub const FLOW_CONFIG_FILE: &str = "config.json";
pub const FLOW_VIEW_FILE: &str = "view.json";
pub const FLOW_HISTORY_DIR: &str = ".history";

pub fn dataflows_dir(home: &Path) -> PathBuf {
    home.join("dataflows")
}

pub fn dataflow_dir(home: &Path, name: &str) -> PathBuf {
    dataflows_dir(home).join(name)
}

pub fn dataflow_yaml_path(dir: &Path) -> PathBuf {
    dir.join(DATAFLOW_FILE)
}

pub fn flow_meta_path(dir: &Path) -> PathBuf {
    dir.join(FLOW_META_FILE)
}

pub fn flow_config_path(dir: &Path) -> PathBuf {
    dir.join(FLOW_CONFIG_FILE)
}

pub fn flow_history_dir(dir: &Path) -> PathBuf {
    dir.join(FLOW_HISTORY_DIR)
}

pub fn flow_view_path(dir: &Path) -> PathBuf {
    dir.join(FLOW_VIEW_FILE)
}
