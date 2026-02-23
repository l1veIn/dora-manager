use crate::config::*;
use tempfile::TempDir;

#[test]
fn resolve_home_with_flag() {
    let result = resolve_home(Some("/tmp/my-dm-home".into())).unwrap();
    assert_eq!(result, std::path::PathBuf::from("/tmp/my-dm-home"));
}

#[test]
fn resolve_home_defaults_to_dot_dm() {
    // Clear DM_HOME in case it's set
    std::env::remove_var("DM_HOME");
    let result = resolve_home(None).unwrap();
    let expected = dirs::home_dir().unwrap().join(".dm");
    assert_eq!(result, expected);
}

#[test]
fn resolve_home_env_override() {
    // Use a unique env var test — note: env vars are process-global, so we use
    // a distinctive value that won't clash.
    std::env::set_var("DM_HOME", "/tmp/dm-env-test-12345");
    let result = resolve_home(None).unwrap();
    assert_eq!(result, std::path::PathBuf::from("/tmp/dm-env-test-12345"));
    std::env::remove_var("DM_HOME");
}

#[test]
fn resolve_home_flag_overrides_env() {
    std::env::set_var("DM_HOME", "/tmp/dm-env-should-be-ignored");
    let result = resolve_home(Some("/tmp/dm-flag-wins".into())).unwrap();
    assert_eq!(result, std::path::PathBuf::from("/tmp/dm-flag-wins"));
    std::env::remove_var("DM_HOME");
}

#[test]
fn versions_dir_path() {
    let home = std::path::PathBuf::from("/home/user/.dm");
    assert_eq!(
        versions_dir(&home),
        std::path::PathBuf::from("/home/user/.dm/versions")
    );
}

#[test]
fn active_link_path() {
    let home = std::path::PathBuf::from("/home/user/.dm");
    assert_eq!(
        active_link(&home),
        std::path::PathBuf::from("/home/user/.dm/active")
    );
}

#[test]
fn config_path_correct() {
    let home = std::path::PathBuf::from("/foo/bar");
    assert_eq!(
        config_path(&home),
        std::path::PathBuf::from("/foo/bar/config.toml")
    );
}

#[test]
fn load_config_missing_file_returns_default() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();
    let cfg = load_config(&home).unwrap();
    assert!(cfg.active_version.is_none());
}

#[test]
fn save_and_load_config_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let cfg = DmConfig {
        active_version: Some("0.4.1".into()),
    };
    save_config(&home, &cfg).unwrap();

    let loaded = load_config(&home).unwrap();
    assert_eq!(loaded.active_version, Some("0.4.1".into()));
}

#[test]
fn save_config_creates_directory() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("nested").join("deep");
    // Directory doesn't exist yet
    assert!(!home.exists());

    let cfg = DmConfig {
        active_version: Some("1.0.0".into()),
    };
    save_config(&home, &cfg).unwrap();

    // Should have created the directory
    assert!(home.exists());
    let loaded = load_config(&home).unwrap();
    assert_eq!(loaded.active_version, Some("1.0.0".into()));
}

#[test]
fn save_config_overwrites_existing() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let cfg1 = DmConfig {
        active_version: Some("0.3.0".into()),
    };
    save_config(&home, &cfg1).unwrap();

    let cfg2 = DmConfig {
        active_version: Some("0.4.1".into()),
    };
    save_config(&home, &cfg2).unwrap();

    let loaded = load_config(&home).unwrap();
    assert_eq!(loaded.active_version, Some("0.4.1".into()));
}

#[test]
fn load_config_default_has_no_active_version() {
    let cfg = DmConfig::default();
    assert!(cfg.active_version.is_none());
}

#[test]
fn config_toml_format_is_valid() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let cfg = DmConfig {
        active_version: Some("0.4.1".into()),
    };
    save_config(&home, &cfg).unwrap();

    let content = std::fs::read_to_string(config_path(&home)).unwrap();
    assert!(content.contains("active_version"));
    assert!(content.contains("0.4.1"));
}
