use tempfile::TempDir;

use crate::config;

/// Helper: create a fake dm home with a version installed
fn setup_fake_home(versions: &[&str], active: Option<&str>) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    // Create version directories with dummy dora binaries
    for ver in versions {
        let ver_dir = config::versions_dir(&home).join(ver);
        std::fs::create_dir_all(&ver_dir).unwrap();
        let bin = ver_dir.join("dora");
        std::fs::write(&bin, "#!/bin/sh\necho dora-cli 0.0.0").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&bin).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&bin, perms).unwrap();
        }
    }

    // Set active version
    if let Some(ver) = active {
        let cfg = config::DmConfig {
            active_version: Some(ver.to_string()),
        };
        config::save_config(&home, &cfg).unwrap();
    }

    tmp
}

// ─── doctor ───

#[tokio::test]
async fn doctor_empty_home() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let report = crate::doctor(&home).await.unwrap();
    assert!(report.installed_versions.is_empty());
    assert!(report.active_version.is_none());
    assert!(!report.active_binary_ok);
    assert!(!report.all_ok);
}

#[tokio::test]
async fn doctor_with_installed_version() {
    let tmp = setup_fake_home(&["0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    let report = crate::doctor(&home).await.unwrap();
    assert_eq!(report.installed_versions.len(), 1);
    assert_eq!(report.installed_versions[0].version, "0.4.1");
    assert!(report.installed_versions[0].active);
    assert_eq!(report.active_version, Some("0.4.1".into()));
    assert!(report.active_binary_ok);
}

#[tokio::test]
async fn doctor_multiple_versions() {
    let tmp = setup_fake_home(&["0.3.9", "0.4.0", "0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    let report = crate::doctor(&home).await.unwrap();
    assert_eq!(report.installed_versions.len(), 3);
    // Should be sorted
    assert_eq!(report.installed_versions[0].version, "0.3.9");
    assert_eq!(report.installed_versions[1].version, "0.4.0");
    assert_eq!(report.installed_versions[2].version, "0.4.1");
    // Only active version should be marked
    assert!(!report.installed_versions[0].active);
    assert!(!report.installed_versions[1].active);
    assert!(report.installed_versions[2].active);
}

#[tokio::test]
async fn doctor_active_but_missing_binary() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();
    // Set active version but don't create the binary
    let cfg = config::DmConfig {
        active_version: Some("0.4.1".into()),
    };
    config::save_config(&home, &cfg).unwrap();

    let report = crate::doctor(&home).await.unwrap();
    assert_eq!(report.active_version, Some("0.4.1".into()));
    assert!(!report.active_binary_ok);
    assert!(!report.all_ok);
}

// ─── versions ───

#[tokio::test]
async fn versions_empty_home() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let report = crate::versions(&home).await.unwrap();
    assert!(report.installed.is_empty());
    // Available may or may not work depending on network
}

#[tokio::test]
async fn versions_with_installed() {
    let tmp = setup_fake_home(&["0.3.9", "0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    let report = crate::versions(&home).await.unwrap();
    assert_eq!(report.installed.len(), 2);
    assert_eq!(report.installed[0].version, "0.3.9");
    assert!(!report.installed[0].active);
    assert_eq!(report.installed[1].version, "0.4.1");
    assert!(report.installed[1].active);
}

// ─── uninstall ───

#[tokio::test]
async fn uninstall_nonexistent_version() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let result = crate::uninstall(&home, "9.9.9").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not installed"));
}

#[tokio::test]
async fn uninstall_active_version_blocked() {
    let tmp = setup_fake_home(&["0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    let result = crate::uninstall(&home, "0.4.1").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Cannot uninstall active"));
}

#[tokio::test]
async fn uninstall_inactive_version_succeeds() {
    let tmp = setup_fake_home(&["0.3.9", "0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    crate::uninstall(&home, "0.3.9").await.unwrap();

    // Verify directory removed
    let ver_dir = config::versions_dir(&home).join("0.3.9");
    assert!(!ver_dir.exists());

    // Active version should still be there
    let ver_dir_active = config::versions_dir(&home).join("0.4.1");
    assert!(ver_dir_active.exists());
}

// ─── use_version ───

#[tokio::test]
async fn use_version_not_installed() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let result = crate::use_version(&home, "0.4.1").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not installed"));
}

#[tokio::test]
async fn use_version_switches_active() {
    let tmp = setup_fake_home(&["0.3.9", "0.4.1"], Some("0.3.9"));
    let home = tmp.path().to_path_buf();

    // Verify current active
    let cfg = config::load_config(&home).unwrap();
    assert_eq!(cfg.active_version, Some("0.3.9".into()));

    // Switch to 0.4.1
    let _ = crate::use_version(&home, "0.4.1").await.unwrap();

    // Verify switched
    let cfg = config::load_config(&home).unwrap();
    assert_eq!(cfg.active_version, Some("0.4.1".into()));
}

#[tokio::test]
async fn use_version_same_version() {
    let tmp = setup_fake_home(&["0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    // Switching to same version should work fine
    let _ = crate::use_version(&home, "0.4.1").await.unwrap();
    let cfg = config::load_config(&home).unwrap();
    assert_eq!(cfg.active_version, Some("0.4.1".into()));
}

// ─── status ───

#[tokio::test]
async fn status_empty_home() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let report = crate::status(&home, false).await.unwrap();
    assert!(report.active_version.is_none());
    assert!(report.actual_version.is_none());
    assert!(!report.runtime_running);
    assert!(report.dataflows.is_empty());
    assert!(report.dm_home.contains(tmp.path().to_str().unwrap()));
}

#[tokio::test]
async fn status_with_active_version() {
    let tmp = setup_fake_home(&["0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    let report = crate::status(&home, false).await.unwrap();
    assert_eq!(report.active_version, Some("0.4.1".into()));
    // The fake binary is a shell script — it runs and returns 0
    // so runtime_running may be true. The important thing is it doesn't crash.
    assert!(report.dm_home.contains(tmp.path().to_str().unwrap()));
}

// ─── dora module ───

#[tokio::test]
async fn active_dora_bin_no_active_version() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    let result = crate::dora::active_dora_bin(&home);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No active dora version"));
}

#[tokio::test]
async fn active_dora_bin_missing_binary() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_path_buf();

    // Set active version but don't create the binary
    let cfg = config::DmConfig {
        active_version: Some("0.4.1".into()),
    };
    config::save_config(&home, &cfg).unwrap();

    let result = crate::dora::active_dora_bin(&home);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("binary not found"));
}

#[tokio::test]
async fn active_dora_bin_found() {
    let tmp = setup_fake_home(&["0.4.1"], Some("0.4.1"));
    let home = tmp.path().to_path_buf();

    let bin = crate::dora::active_dora_bin(&home).unwrap();
    assert!(bin.exists());
    assert!(bin.ends_with("dora"));
}
