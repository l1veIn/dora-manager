use tempfile::tempdir;

use crate::test_support::{env_lock, set_path};

use super::*;

#[cfg(not(target_os = "windows"))]
fn write_executable(path: &std::path::Path, content: &str) {
    std::fs::write(path, content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }
}

fn write_sample_service(root: &std::path::Path, id: &str) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("service.json"),
        format!(
            r#"{{
              "id": "{id}",
              "name": "Sample Service",
              "version": "0.1.0",
              "description": "Sample service",
              "scope": "global",
              "runtime": {{"kind": "command", "exec": "python service.py"}},
              "files": {{"readme": "README.md"}},
              "methods": [
                {{"name": "echo", "input_schema": {{"type": "object"}}, "output_schema": {{"type": "object"}}}}
              ]
            }}"#
        ),
    )
    .unwrap();
    std::fs::write(root.join("README.md"), "# Sample Service\n").unwrap();
    std::fs::write(root.join("service.py"), "print('ok')\n").unwrap();
}

#[test]
fn list_services_includes_builtins() {
    let dir = tempdir().unwrap();
    let services = list_services(dir.path()).unwrap();

    assert!(services.iter().any(|service| service.id == "message"));
    assert!(services.iter().any(|service| service.id == "registry"));
}

#[test]
fn get_builtin_service() {
    let dir = tempdir().unwrap();
    let service = get_service(dir.path(), "message").unwrap().unwrap();

    assert_eq!(service.id, "message");
    assert_eq!(service.scope, ServiceScope::Run);
    assert!(service.methods.iter().any(|method| method.name == "send"));
}

#[test]
fn list_services_reads_service_json() {
    let dir = tempdir().unwrap();
    let service_path = service_dir(dir.path(), "calculator");
    std::fs::create_dir_all(&service_path).unwrap();
    std::fs::write(
        service_json_path(dir.path(), "calculator"),
        r#"{
          "id": "calculator",
          "name": "Calculator",
          "version": "0.1.0",
          "description": "Simple math helpers",
          "scope": "global",
          "runtime": {"kind": "command", "exec": "python service.py"},
          "methods": [
            {
              "name": "add",
              "description": "Add two numbers",
              "input_schema": {"type": "object"},
              "output_schema": {"type": "object"}
            }
          ]
        }"#,
    )
    .unwrap();

    let services = list_services(dir.path()).unwrap();
    let service = services
        .iter()
        .find(|service| service.id == "calculator")
        .unwrap();

    assert_eq!(service.display_name(), "Calculator");
    assert_eq!(service.runtime.kind, ServiceRuntimeKind::Command);
    assert_eq!(service.methods[0].name, "add");
}

#[test]
fn builtin_service_readme_is_readable() {
    let dir = tempdir().unwrap();
    let readme = get_service_readme(dir.path(), "message").unwrap();
    assert!(readme.contains("Message Service"));
}

#[test]
fn create_service_scaffolds_workspace_and_config_roundtrips() {
    let dir = tempdir().unwrap();
    let service = create_service(dir.path(), "demo-service", "Demo service").unwrap();

    assert_eq!(service.id, "demo-service");
    assert!(service.path.join("service.json").exists());
    assert!(service.path.join("pyproject.toml").exists());
    assert!(service.path.join("demo_service/main.py").exists());

    let config = serde_json::json!({"model": "tiny"});
    save_service_config(dir.path(), "demo-service", &config).unwrap();
    let loaded = get_service_config(dir.path(), "demo-service").unwrap();
    assert_eq!(loaded["model"], "tiny");
}

#[test]
#[cfg(not(target_os = "windows"))]
fn install_service_creates_venv_and_updates_manifest() {
    let _guard = env_lock();
    let dir = tempdir().unwrap();
    let home = dir.path();
    let bin_dir = home.join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    write_executable(
        &bin_dir.join("uv"),
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo uv 0.1; exit 0; fi\nif [ \"$1\" = \"venv\" ]; then /bin/mkdir -p \"$2/bin\"; exit 0; fi\nif [ \"$1\" = \"pip\" ]; then exit 0; fi\nexit 1\n",
    );

    create_service(home, "demo-service", "Demo service").unwrap();
    let _path = set_path(bin_dir);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let service = rt.block_on(install_service(home, "demo-service")).unwrap();

    assert_eq!(
        service.runtime.exec.as_deref(),
        Some(".venv/bin/demo-service")
    );
    assert!(!service.installed_at.is_empty());

    let persisted = get_service(home, "demo-service").unwrap().unwrap();
    assert_eq!(
        persisted.runtime.exec.as_deref(),
        Some(".venv/bin/demo-service")
    );
}

#[test]
fn import_local_lists_files_and_uninstalls() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    write_sample_service(&source, "sample");

    let service = import_local(dir.path(), "sample", &source).unwrap();
    assert_eq!(service.id, "sample");

    let readme = get_service_readme(dir.path(), "sample").unwrap();
    assert!(readme.contains("Sample Service"));

    let files = git_like_file_tree(dir.path(), "sample").unwrap();
    assert!(files.iter().any(|file| file == "service.json"));
    assert!(files.iter().any(|file| file == "service.py"));

    let content = read_service_file(dir.path(), "sample", "service.py").unwrap();
    assert!(content.contains("print"));

    uninstall_service(dir.path(), "sample").unwrap();
    assert!(get_service(dir.path(), "sample").unwrap().is_none());
}

#[test]
fn import_local_rejects_mismatched_manifest_id() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    write_sample_service(&source, "actual");

    let err = import_local(dir.path(), "requested", &source)
        .unwrap_err()
        .to_string();
    assert!(err.contains("does not match requested id"));
}

#[test]
fn uninstall_builtin_service_is_rejected() {
    let dir = tempdir().unwrap();
    let err = uninstall_service(dir.path(), "message")
        .unwrap_err()
        .to_string();
    assert!(err.contains("builtin"));
}

#[test]
fn read_service_file_rejects_traversal() {
    let dir = tempdir().unwrap();
    let err = read_service_file(dir.path(), "message", "../Cargo.toml")
        .unwrap_err()
        .to_string();
    assert!(err.contains("Invalid service file path"));
}

#[test]
fn get_service_returns_none_for_missing_service() {
    let dir = tempdir().unwrap();
    let service = get_service(dir.path(), "missing").unwrap();
    assert!(service.is_none());
}
