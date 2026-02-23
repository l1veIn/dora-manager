use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn dm_cmd() -> Command {
    let mut cmd = Command::cargo_bin("dm").expect("binary dm should build");
    cmd.env("NO_COLOR", "1");
    cmd
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
fn graph_validate_reports_parse_error_for_invalid_yaml() {
    let home = tempdir().unwrap();
    let graph_file = home.path().join("bad.yml");
    fs::write(&graph_file, "nodes: [\n").unwrap();

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "graph",
            "validate",
            graph_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse YAML graph file"));
}

#[test]
fn graph_run_placeholder_message() {
    let home = tempdir().unwrap();

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "graph", "run", "graph.yml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Not implemented yet"));
}
