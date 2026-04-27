//! Service manager - discover and describe operation-oriented dm services.
//!
//! Services are installed in `~/.dm/services/<id>/` with metadata stored in
//! `service.json`. Built-in services may also be provided by dm itself.

mod import;
mod install;
mod local;
mod model;
mod paths;

#[cfg(test)]
mod tests;

pub use import::{import_git, import_local};
pub use install::install_service;
pub use local::{
    create_service, get_service, get_service_config, get_service_readme, git_like_file_tree,
    list_services, read_service_file, read_service_file_bytes, save_service_config, service_status,
    uninstall_service,
};
pub use model::{
    Service, ServiceDisplay, ServiceExample, ServiceFiles, ServiceMaintainer, ServiceMethod,
    ServiceRepository, ServiceRuntime, ServiceRuntimeKind, ServiceScope, ServiceStatus,
};
pub use paths::{
    builtin_services_dir, configured_service_dirs, resolve_service_dir, resolve_service_json_path,
    service_dir, service_json_path,
};

pub(crate) fn current_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
