//! Node Manager - Install, list, and manage local dora nodes
//!
//! Nodes are installed in `~/.dm/nodes/<id>/` with metadata stored in `dm.json`.

mod import;
pub(crate) mod init;
mod install;
mod local;
mod model;
mod paths;

#[cfg(test)]
mod tests;

pub use import::{import_git, import_local};
pub use install::install_node;
pub use local::{
    create_node, get_node_config, get_node_readme, list_nodes, node_status, save_node_config,
    uninstall_node,
};
pub use model::{Node, NodeSource};
pub use paths::{dm_json_path, is_managed_node, node_dir, resolve_dm_json_path, resolve_node_dir};

pub(crate) fn current_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
