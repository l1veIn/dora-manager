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

#[cfg(not(target_os = "windows"))]
const FAKE_DORA_UUID: &str = "019cc181-adad-7654-aa78-63502362337b";

#[cfg(not(target_os = "windows"))]
fn setup_fake_runtime(home: &std::path::Path, active_version: &str) {
    let version_dir = home.join("versions").join(active_version);
    fs::create_dir_all(&version_dir).unwrap();

    #[cfg(target_os = "windows")]
    let bin_name = "dora.exe";
    #[cfg(not(target_os = "windows"))]
    let bin_name = "dora";

    let bin = version_dir.join(bin_name);
    let state_file = home.join("active_dataflow_id");
    fs::write(
        &bin,
        format!(
            r#"#!/bin/sh
cmd="$1"
case "$cmd" in
  check)
    exit 0
    ;;
  list)
    if [ -f "{state_file}" ]; then
      echo "UUID Name Status Nodes CPU Memory"
      printf "%s test-flow Running 1 0.0%% 0.0\\ GB\\n" "$(cat "{state_file}")"
    fi
    exit 0
    ;;
  start)
    run_yaml="$2"
    run_dir="$(dirname "$run_yaml")"
    mkdir -p "$run_dir/out/{fake_uuid}"
    echo "worker log line" > "$run_dir/out/{fake_uuid}/log_worker.txt"
    echo "{fake_uuid}" > "{state_file}"
    echo "dataflow started: {fake_uuid}"
    exit 0
    ;;
  stop)
    rm -f "{state_file}"
    echo "stopped"
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
"#,
            state_file = state_file.display(),
            fake_uuid = FAKE_DORA_UUID
        ),
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
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

#[test]
fn node_list_includes_builtin_nodes() {
    let home = tempdir().unwrap();

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "node", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dm-test-media-capture"))
        .stdout(predicate::str::contains("dm-test-audio-capture"));
}

#[test]
fn node_uninstall_missing_node_shows_friendly_error() {
    let home = tempdir().unwrap();

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "node",
            "uninstall",
            "missing-node",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing-node"))
        .stderr(predicate::str::contains("1 node(s) failed to uninstall"));
}

#[test]
#[cfg(not(target_os = "windows"))]
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
        .stderr(predicate::str::contains("is not executable"));
}

#[test]
fn start_fails_gracefully_when_no_dora_installed() {
    let home = tempdir().unwrap();

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "start",
            "graph.yml",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No active dora version"));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn start_creates_run_and_runs_list_shows_it() {
    let home = tempdir().unwrap();
    setup_fake_runtime(home.path(), "0.4.1");
    let graph_file = home.path().join("ok.yml");
    fs::write(&graph_file, "nodes: []\n").unwrap();

    let output = dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "start",
            graph_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Run created:"));

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "runs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ok"))
        .stdout(predicate::str::contains("⏳"));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn runs_logs_and_stop_work_for_started_run() {
    let home = tempdir().unwrap();
    setup_fake_runtime(home.path(), "0.4.1");
    let graph_file = home.path().join("ok.yml");
    fs::write(&graph_file, "nodes: []\n").unwrap();

    let output = dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "start",
            graph_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let run_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("✅ Run created: "))
        .map(str::trim)
        .unwrap()
        .to_string();

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "runs",
            "logs",
            &run_id,
            "worker",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("worker log line"));

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "runs",
            "stop",
            &run_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped run"));

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "runs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("✅"));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn start_rejects_conflicting_active_run_without_force() {
    let home = tempdir().unwrap();
    setup_fake_runtime(home.path(), "0.4.1");
    let graph_file = home.path().join("ok.yml");
    fs::write(&graph_file, "nodes: []\n").unwrap();

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "start",
            graph_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "start",
            graph_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already running as run"));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn runs_refresh_marks_stale_running_run_as_stopped() {
    let home = tempdir().unwrap();
    setup_fake_runtime(home.path(), "0.4.1");
    let graph_file = home.path().join("ok.yml");
    fs::write(&graph_file, "nodes: []\n").unwrap();

    let output = dm_cmd()
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "start",
            graph_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    fs::remove_file(home.path().join("active_dataflow_id")).unwrap();

    dm_cmd()
        .args(["--home", home.path().to_str().unwrap(), "runs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("✅"))
        .stdout(predicate::str::contains("ok"));
}
