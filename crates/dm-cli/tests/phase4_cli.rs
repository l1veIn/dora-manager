use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn dm_cmd() -> Command {
    let mut cmd = cargo_bin_cmd!("dm");
    cmd.env("NO_COLOR", "1");
    cmd
}

fn setup_fake_runtime(home: &std::path::Path, active_version: &str) {
    let version_dir = home.join("versions").join(active_version);
    fs::create_dir_all(&version_dir).unwrap();

    let bin = version_dir.join("dora");
    fs::write(
        &bin,
        r#"#!/bin/sh
cmd="$1"
case "$cmd" in
  check)
    exit 0
    ;;
  start)
    echo "started"
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).unwrap();
    }

    fs::write(
        home.join("config.toml"),
        format!("active_version = \"{}\"\n", active_version),
    )
    .unwrap();
}

#[test]
fn node_install_requires_id() {
    dm_cmd()
        .args(["node", "install"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn node_list_empty_home() {
    let home = tempdir().unwrap();

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "node", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No nodes installed"));
}

#[test]
fn node_uninstall_missing_node_shows_friendly_error() {
    let home = tempdir().unwrap();

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "node", "uninstall", "missing-node"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to uninstall node 'missing-node'"));
}

#[test]
fn start_reports_parse_error_for_invalid_yaml() {
    let home = tempdir().unwrap();
    setup_fake_runtime(home.path(), "0.4.1");
    let graph_file = home.path().join("bad.yml");
    fs::write(&graph_file, "nodes: [\n").unwrap();

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "start",
            graph_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to transpile"));
}

#[test]
fn start_requires_runtime_to_be_running() {
    let home = tempdir().unwrap();

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "start", "graph.yml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Dora runtime is not running"));
}
