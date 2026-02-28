use std::path::{Path, PathBuf};

pub(crate) fn nodes_dir(home: &Path) -> PathBuf {
    home.join("nodes")
}

pub fn node_dir(home: &Path, id: &str) -> PathBuf {
    nodes_dir(home).join(id)
}

pub fn dm_json_path(home: &Path, id: &str) -> PathBuf {
    node_dir(home, id).join("dm.json")
}
