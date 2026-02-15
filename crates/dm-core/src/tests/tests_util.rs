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

#[tokio::test]
async fn get_command_version_works() {
    // Test with `echo` which should return its argument
    let result = util::get_command_version("echo", &["test-version-1.0"]).await;
    assert!(result.is_some());
    assert!(result.unwrap().contains("test-version-1.0"));
}

#[tokio::test]
async fn get_command_version_nonexistent() {
    let result =
        util::get_command_version("nonexistent-command-xyz-999", &["--version"]).await;
    assert!(result.is_none());
}
