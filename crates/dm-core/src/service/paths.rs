use std::path::{Path, PathBuf};

pub fn services_dir(home: &Path) -> PathBuf {
    home.join("services")
}

pub fn builtin_services_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../services")
}

pub fn configured_service_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    push_unique(&mut dirs, services_dir(home));
    push_unique(&mut dirs, builtin_services_dir());

    if let Some(extra) = std::env::var_os("DM_SERVICE_DIRS") {
        for dir in std::env::split_paths(&extra) {
            push_unique(&mut dirs, dir);
        }
    }

    dirs
}

pub fn service_dir(home: &Path, id: &str) -> PathBuf {
    services_dir(home).join(id)
}

pub fn service_json_path(home: &Path, id: &str) -> PathBuf {
    service_dir(home, id).join("service.json")
}

pub fn resolve_service_dir(home: &Path, id: &str) -> Option<PathBuf> {
    configured_service_dirs(home)
        .into_iter()
        .map(|dir| dir.join(id))
        .find(|path| path.exists())
}

pub fn resolve_service_json_path(home: &Path, id: &str) -> Option<PathBuf> {
    resolve_service_dir(home, id).map(|dir| dir.join("service.json"))
}

fn push_unique(dirs: &mut Vec<PathBuf>, path: PathBuf) {
    if !dirs.iter().any(|existing| existing == &path) {
        dirs.push(path);
    }
}
