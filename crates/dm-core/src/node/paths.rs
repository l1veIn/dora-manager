use std::path::{Path, PathBuf};

pub(crate) fn nodes_dir(home: &Path) -> PathBuf {
    home.join("nodes")
}

pub(crate) fn builtin_nodes_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../nodes")
}

pub(crate) fn configured_node_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    push_unique(&mut dirs, nodes_dir(home));
    push_unique(&mut dirs, builtin_nodes_dir());

    if let Some(extra) = std::env::var_os("DM_NODE_DIRS") {
        for dir in std::env::split_paths(&extra) {
            push_unique(&mut dirs, dir);
        }
    }

    dirs
}

pub fn node_dir(home: &Path, id: &str) -> PathBuf {
    nodes_dir(home).join(id)
}

pub fn dm_json_path(home: &Path, id: &str) -> PathBuf {
    node_dir(home, id).join("dm.json")
}

pub fn resolve_node_dir(home: &Path, id: &str) -> Option<PathBuf> {
    configured_node_dirs(home)
        .into_iter()
        .map(|dir| dir.join(id))
        .find(|path| path.exists())
}

pub fn resolve_dm_json_path(home: &Path, id: &str) -> Option<PathBuf> {
    resolve_node_dir(home, id).map(|dir| dir.join("dm.json"))
}

pub fn is_managed_node(home: &Path, id: &str) -> bool {
    node_dir(home, id).exists()
}

fn push_unique(dirs: &mut Vec<PathBuf>, path: PathBuf) {
    if !dirs.iter().any(|existing| existing == &path) {
        dirs.push(path);
    }
}
