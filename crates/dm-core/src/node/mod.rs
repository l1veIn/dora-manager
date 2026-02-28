//! Node Manager - Install, list, and manage local dora nodes
//!
//! Nodes are installed in `~/.dm/nodes/<id>/` with metadata stored in `dm.json`.

mod download;
mod install;
mod local;
mod model;
mod paths;

#[cfg(test)]
mod tests;

pub use download::download_node;
pub use install::install_node;
pub use local::{
    create_node, get_node_config, get_node_readme, list_nodes, node_status, save_node_config,
    uninstall_node,
};
pub use model::{NodeEntry, NodeMetaFile, NodeSource};
pub use paths::{dm_json_path, node_dir};

pub(crate) fn current_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
