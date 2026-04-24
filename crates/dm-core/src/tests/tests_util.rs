use crate::test_support::env_lock;
use crate::util;

#[test]
fn human_size_bytes() {
    assert_eq!(util::human_size(0), "0 B");
    assert_eq!(util::human_size(100), "100 B");
    assert_eq!(util::human_size(512), "512 B");
    assert_eq!(util::human_size(1023), "1023 B");
}

#[test]
fn human_size_kib() {
    assert_eq!(util::human_size(1024), "1.0 KiB");
    assert_eq!(util::human_size(1536), "1.5 KiB");
    assert_eq!(util::human_size(10240), "10.0 KiB");
}

#[test]
fn human_size_mib() {
    assert_eq!(util::human_size(1024 * 1024), "1.0 MiB");
    assert_eq!(util::human_size(5 * 1024 * 1024), "5.0 MiB");
    assert_eq!(util::human_size(5_500_000), "5.2 MiB");
}

#[test]
fn check_command_found() {
    let _guard = env_lock();
    // `sh` should exist on all unix systems
    let result = util::check_command("sh");
    assert!(result.is_some());
    assert!(result.unwrap().contains("sh"));
}

#[test]
fn check_command_not_found() {
    let result = util::check_command("this-command-definitely-does-not-exist-xyz-123");
    assert!(result.is_none());
}

#[test]
fn resolve_dm_cli_exe_prefers_env_override() {
    let _guard = env_lock();
    let original = std::env::var_os(util::DM_CLI_BIN_ENV_KEY);
    std::env::set_var(util::DM_CLI_BIN_ENV_KEY, "/tmp/custom-dm");

    assert_eq!(
        util::resolve_dm_cli_exe(),
        std::path::PathBuf::from("/tmp/custom-dm")
    );

    if let Some(value) = original {
        std::env::set_var(util::DM_CLI_BIN_ENV_KEY, value);
    } else {
        std::env::remove_var(util::DM_CLI_BIN_ENV_KEY);
    }
}

#[test]
#[cfg(not(target_os = "windows"))]
fn resolve_dm_cli_exe_uses_path_before_fallback() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = env_lock();
    let original_dm_cli_bin = std::env::var_os(util::DM_CLI_BIN_ENV_KEY);
    std::env::remove_var(util::DM_CLI_BIN_ENV_KEY);

    let tmp = tempfile::TempDir::new().unwrap();
    let dm = tmp.path().join("dm");
    std::fs::write(&dm, "#!/bin/sh\nexit 0\n").unwrap();
    let mut perms = std::fs::metadata(&dm).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&dm, perms).unwrap();
    let _path_guard = crate::test_support::set_path(tmp.path().as_os_str());

    assert_eq!(util::resolve_dm_cli_exe(), dm);

    if let Some(value) = original_dm_cli_bin {
        std::env::set_var(util::DM_CLI_BIN_ENV_KEY, value);
    } else {
        std::env::remove_var(util::DM_CLI_BIN_ENV_KEY);
    }
}

#[test]
fn is_valid_dora_binary_nonexistent() {
    let path = std::path::PathBuf::from("/tmp/nonexistent-dora-binary-xyz");
    assert!(!util::is_valid_dora_binary(&path));
}

#[test]
fn is_valid_dora_binary_directory() {
    let tmp = tempfile::TempDir::new().unwrap();
    // A directory is not a valid binary
    assert!(!util::is_valid_dora_binary(tmp.path()));
}

#[test]
fn is_valid_dora_binary_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let file = tmp.path().join("dora");
    std::fs::write(&file, "#!/bin/sh\necho hello").unwrap();
    assert!(util::is_valid_dora_binary(&file));
}

#[test]
fn get_command_version_works() {
    let _guard = env_lock();
    // Test with `echo` which should return its argument
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(util::get_command_version("echo", &["test-version-1.0"]));
    assert!(result.is_some());
    assert!(result.unwrap().contains("test-version-1.0"));
}

#[tokio::test]
async fn get_command_version_nonexistent() {
    let result = util::get_command_version("nonexistent-command-xyz-999", &["--version"]).await;
    assert!(result.is_none());
}
