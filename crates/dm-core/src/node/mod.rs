//! Node Manager - Install, list, and manage local dora nodes
//!
//! Nodes are installed in `~/.dm/nodes/<id>/` with metadata stored in `dm.json`.

pub mod hub;
mod import;
pub(crate) mod init;
mod install;
mod local;
mod model;
mod paths;
pub mod schema;

#[cfg(test)]
mod tests;

pub use import::{import_git, import_local};
pub use install::install_node;
pub use local::{
    create_node, get_node_config, get_node_readme, git_like_file_tree, list_nodes, node_status,
    read_node_file, save_node_config, uninstall_node,
};
pub use model::{
    Node, NodeDisplay, NodeExample, NodeFiles, NodeMaintainer, NodePort, NodePortDirection,
    NodeRepository, NodeRuntime, NodeSource,
};
pub use paths::{dm_json_path, is_managed_node, node_dir, resolve_dm_json_path, resolve_node_dir};

pub(crate) fn current_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
