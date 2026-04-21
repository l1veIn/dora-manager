#[cfg(test)]
mod tests {
    use std::fs;
    use std::future::Future;
    use std::path::Path;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    use anyhow::{anyhow, Result};

    use crate::config;
    use crate::node::{node_dir, Node, NodeDisplay, NodeFiles, NodeRuntime, NodeSource};
    use crate::runs::model::{
        RunInstance, RunSource, RunStatus, StartConflictStrategy, TerminationReason,
    };
    use crate::runs::repo;
    use crate::runs::runtime::{RuntimeBackend, RuntimeDataflow, STOP_TIMEOUT_SECS};
    use crate::runs::service::{service_query, service_runtime, service_start};
    use crate::runs::state::build_outcome;

    #[derive(Clone)]
    struct TestBackend {
        start_result: std::result::Result<(Option<String>, String), String>,
        stop_result: std::result::Result<(), String>,
        list_result: std::result::Result<Vec<RuntimeDataflow>, String>,
        stop_calls: Arc<Mutex<Vec<String>>>,
    }

    impl RuntimeBackend for TestBackend {
        fn start_detached<'a>(
            &'a self,
            _home: &'a Path,
            _transpiled_path: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<(Option<String>, String)>> + Send + 'a>> {
            Box::pin(async move { self.start_result.clone().map_err(|err| anyhow!(err)) })
        }

        fn stop<'a>(
            &'a self,
            _home: &'a Path,
            dora_uuid: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
            let dora_uuid = dora_uuid.to_string();
            let stop_calls = Arc::clone(&self.stop_calls);
            Box::pin(async move {
                stop_calls.lock().unwrap().push(dora_uuid);
                self.stop_result.clone().map_err(|err| anyhow!(err))
            })
        }

        fn list(&self, _home: &Path) -> Result<Vec<RuntimeDataflow>> {
            self.list_result.clone().map_err(|err| anyhow!(err))
        }
    }

    fn setup_managed_node(home: &Path, id: &str, executable: &str) {
        let dir = node_dir(home, id);
        fs::create_dir_all(&dir).unwrap();

        let exec_path = dir.join(executable);
        if let Some(parent) = exec_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&exec_path, "#!/bin/sh\nexit 0\n").unwrap();

        let meta = Node {
            id: id.to_string(),
            name: id.to_string(),
            version: "1.0.0".to_string(),
            installed_at: "2026-03-09T00:00:00Z".to_string(),
            source: NodeSource {
                build: "pip install test-node".to_string(),
                github: None,
            },
            description: String::new(),
            executable: executable.to_string(),
            repository: None,
            maintainers: Vec::new(),
            license: None,
            display: NodeDisplay::default(),
            dm: None,
            capabilities: Vec::new(),
            runtime: NodeRuntime::default(),
            ports: Vec::new(),
            files: NodeFiles::default(),
            examples: Vec::new(),
            config_schema: None,
            dynamic_ports: false,
            interaction: None,
            path: Default::default(),
        };

        fs::write(
            dir.join("dm.json"),
            serde_json::to_string_pretty(&meta).unwrap(),
        )
        .unwrap();
    }

    fn write_running_run(home: &Path, run_id: &str, dora_uuid: Option<&str>) {
        let run = RunInstance {
            run_id: run_id.to_string(),
            dora_uuid: dora_uuid.map(str::to_string),
            dataflow_name: "demo".to_string(),
            dataflow_hash: "sha256:test".to_string(),
            started_at: "2026-03-09T00:00:00Z".to_string(),
            outcome: build_outcome(RunStatus::Running, None, None, None),
            ..RunInstance::default()
        };
        repo::create_layout(home, run_id).unwrap();
        repo::save_run(home, &run).unwrap();
    }

    fn write_run(home: &Path, run: RunInstance) {
        repo::create_layout(home, &run.run_id).unwrap();
        repo::save_run(home, &run).unwrap();
    }

    fn write_runtime_log(home: &Path, run_id: &str, dora_uuid: &str, node_id: &str, content: &str) {
        let out_dir = repo::run_out_dir(home, run_id).join(dora_uuid);
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(out_dir.join(format!("log_{node_id}.txt")), content).unwrap();
    }

    fn setup_fake_runtime_home(home: &Path, active_version: &str, script: &str) {
        let version_dir = config::versions_dir(home).join(active_version);
        fs::create_dir_all(&version_dir).unwrap();

        let bin = version_dir.join(config::dora_bin_name());
        fs::write(&bin, script).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&bin).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin, perms).unwrap();
        }

        config::save_config(
            home,
            &config::DmConfig {
                active_version: Some(active_version.to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[tokio::test]
    async fn start_run_fails_when_runtime_uuid_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        setup_managed_node(home, "test-node", ".venv/bin/test-node");

        let backend = TestBackend {
            start_result: Ok((None, "started without uuid".to_string())),
            stop_result: Ok(()),
            list_result: Ok(Vec::new()),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        let err = service_start::start_run_from_yaml_with_source_and_strategy_and_backend(
            home,
            "nodes:\n  - id: n1\n    node: test-node\n",
            "demo",
            None,
            RunSource::Cli,
            StartConflictStrategy::Fail,
            &backend,
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("did not return a runtime UUID"));

        let runs = repo::list_run_instances(home).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, RunStatus::Failed);
        assert_eq!(
            runs[0].termination_reason,
            Some(TerminationReason::StartFailed)
        );
        assert_eq!(runs[0].dora_uuid, None);
    }

    #[test]
    fn refresh_run_statuses_keeps_running_state_when_runtime_list_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-1", Some("uuid-1"));
        let mut runs = repo::list_run_instances(home).unwrap();

        let backend = TestBackend {
            start_result: Ok((Some("uuid-1".to_string()), "started".to_string())),
            stop_result: Ok(()),
            list_result: Err("transient dora list failure".to_string()),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        service_runtime::refresh_run_statuses_with_backend(home, &mut runs, &backend).unwrap();

        assert_eq!(runs[0].status, RunStatus::Running);
        assert_eq!(runs[0].termination_reason, None);
        assert_eq!(runs[0].dora_uuid.as_deref(), Some("uuid-1"));

        let persisted = repo::load_run(home, "run-1").unwrap();
        assert_eq!(persisted.status, RunStatus::Running);
        assert_eq!(persisted.termination_reason, None);
        assert_eq!(persisted.runtime_observed_at, None);
        assert_eq!(persisted.failure_reason, None);
    }

    #[test]
    fn refresh_run_statuses_reconciles_stale_running_state_when_runtime_check_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-1", Some("uuid-1"));
        setup_fake_runtime_home(
            home,
            "0.4.1",
            r#"#!/bin/sh
cmd="$1"
case "$cmd" in
  check)
    echo "Runtime unavailable" >&2
    exit 1
    ;;
  list)
    echo "Could not connect to dora-coordinator" >&2
    exit 1
    ;;
  *)
    exit 0
    ;;
esac
"#,
        );

        let runs = service_runtime::refresh_run_statuses(home).unwrap();

        assert_eq!(runs[0].status, RunStatus::Stopped);
        assert_eq!(
            runs[0].termination_reason,
            Some(TerminationReason::RuntimeLost)
        );
        assert_eq!(runs[0].failure_reason.as_deref(), Some("runtime_lost"));

        let persisted = repo::load_run(home, "run-1").unwrap();
        assert_eq!(persisted.status, RunStatus::Stopped);
        assert_eq!(
            persisted.termination_reason,
            Some(TerminationReason::RuntimeLost)
        );
        assert_eq!(persisted.failure_reason.as_deref(), Some("runtime_lost"));
        assert!(persisted.runtime_observed_at.is_some());
    }

    #[test]
    fn mark_stop_requested_persists_background_stop_state() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-stop-marked", Some("uuid-stop-marked"));

        let run = service_runtime::mark_stop_requested(home, "run-stop-marked").unwrap();
        assert_eq!(run.status, RunStatus::Running);
        assert!(run.stop_request.requested_at.is_some());
        assert_eq!(run.stop_request.last_error, None);

        let persisted = repo::load_run(home, "run-stop-marked").unwrap();
        assert_eq!(persisted.status, RunStatus::Running);
        assert!(persisted.stop_request.requested_at.is_some());
        assert_eq!(persisted.stop_request.last_error, None);
    }

    #[tokio::test]
    async fn stop_run_success_marks_run_stopped_and_syncs_logs() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-stop-ok", Some("uuid-stop-ok"));
        write_runtime_log(
            home,
            "run-stop-ok",
            "uuid-stop-ok",
            "worker",
            "worker log line",
        );

        let backend = TestBackend {
            start_result: Ok((Some("unused".to_string()), "started".to_string())),
            stop_result: Ok(()),
            list_result: Ok(Vec::new()),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        let run = service_runtime::stop_run_with_backend(home, "run-stop-ok", &backend)
            .await
            .unwrap();
        assert_eq!(run.status, RunStatus::Stopped);
        assert_eq!(
            run.termination_reason,
            Some(TerminationReason::StoppedByUser)
        );
        assert_eq!(run.stop_request.requested_at, None);
        assert_eq!(run.stop_request.last_error, None);
        assert_eq!(run.node_count_observed, 1);
        assert_eq!(run.nodes_observed, vec!["worker".to_string()]);
        assert_eq!(
            fs::read_to_string(
                repo::run_out_dir(home, "run-stop-ok")
                    .join("uuid-stop-ok")
                    .join("log_worker.txt")
            )
            .unwrap(),
            "worker log line"
        );
        assert_eq!(
            backend.stop_calls.lock().unwrap().as_slice(),
            &["uuid-stop-ok".to_string()]
        );
    }

    #[test]
    fn get_run_includes_expected_nodes_before_live_logs_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();

        let run = RunInstance {
            run_id: "run-expected-nodes".to_string(),
            dora_uuid: Some("uuid-expected-nodes".to_string()),
            dataflow_name: "demo".to_string(),
            dataflow_hash: "sha256:test".to_string(),
            started_at: "2026-03-09T00:00:00Z".to_string(),
            node_count_expected: 3,
            nodes_expected: vec![
                "__dm_bridge".to_string(),
                "display".to_string(),
                "echo".to_string(),
            ],
            outcome: build_outcome(RunStatus::Running, None, None, None),
            ..RunInstance::default()
        };

        write_run(home, run);

        let detail = service_query::get_run(home, "run-expected-nodes").unwrap();
        let node_ids = detail
            .nodes
            .into_iter()
            .map(|node| node.id)
            .collect::<Vec<_>>();

        assert_eq!(node_ids, vec!["__dm_bridge", "display", "echo"]);
    }

    #[test]
    fn read_run_log_reads_live_dora_output_without_logs_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-live-log-read", Some("uuid-live-log-read"));
        write_runtime_log(
            home,
            "run-live-log-read",
            "uuid-live-log-read",
            "worker",
            "worker live line",
        );

        let content = service_query::read_run_log(home, "run-live-log-read", "worker").unwrap();

        assert_eq!(content, "worker live line");
        assert!(!repo::run_logs_dir(home, "run-live-log-read").exists());
    }

    #[tokio::test]
    async fn stop_run_failure_tolerates_when_not_in_runtime_list() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-stop-tol", Some("uuid-stop-tol"));

        // stop() fails but list() returns empty → dataflow is gone
        let backend = TestBackend {
            start_result: Ok((Some("unused".to_string()), "started".to_string())),
            stop_result: Err("Node worker failed: boom".to_string()),
            list_result: Ok(Vec::new()),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        let run = service_runtime::stop_run_with_backend(home, "run-stop-tol", &backend)
            .await
            .unwrap();
        assert_eq!(run.status, RunStatus::Stopped);
        assert_eq!(
            run.termination_reason,
            Some(TerminationReason::StoppedByUser)
        );
    }

    #[tokio::test]
    async fn stop_run_failure_marks_run_failed_when_still_running() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-stop-fail", Some("uuid-stop-fail"));

        // stop() fails AND list() still shows it running → true failure
        let backend = TestBackend {
            start_result: Ok((Some("unused".to_string()), "started".to_string())),
            stop_result: Err("Node worker failed: boom".to_string()),
            list_result: Ok(vec![RuntimeDataflow {
                id: "uuid-stop-fail".to_string(),
                status: RunStatus::Running,
            }]),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        let err = service_runtime::stop_run_with_backend(home, "run-stop-fail", &backend)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Node worker failed: boom"));

        let persisted = repo::load_run(home, "run-stop-fail").unwrap();
        assert_eq!(persisted.status, RunStatus::Failed);
        assert_eq!(
            persisted.termination_reason,
            Some(TerminationReason::NodeFailed)
        );
        assert_eq!(persisted.failure_node.as_deref(), Some("worker"));
        assert_eq!(persisted.failure_message.as_deref(), Some("boom"));
    }

    #[tokio::test]
    async fn stop_run_timeout_keeps_run_running_when_runtime_still_reports_running() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        write_running_run(home, "run-stop-timeout", Some("uuid-stop-timeout"));

        let backend = TestBackend {
            start_result: Ok((Some("unused".to_string()), "started".to_string())),
            stop_result: Err(format!("dora stop timed out after {}s", STOP_TIMEOUT_SECS)),
            list_result: Ok(vec![RuntimeDataflow {
                id: "uuid-stop-timeout".to_string(),
                status: RunStatus::Running,
            }]),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        let err = service_runtime::stop_run_with_backend(home, "run-stop-timeout", &backend)
            .await
            .unwrap_err();
        assert!(err
            .to_string()
            .contains(&format!("dora stop timed out after {}s", STOP_TIMEOUT_SECS)));

        let persisted = repo::load_run(home, "run-stop-timeout").unwrap();
        assert_eq!(persisted.status, RunStatus::Running);
        assert_eq!(persisted.termination_reason, None);
        assert_eq!(persisted.failure_node, None);
        assert_eq!(persisted.failure_message, None);
        assert!(persisted.runtime_observed_at.is_some());
        assert!(persisted.stop_request.requested_at.is_some());
        assert!(persisted
            .stop_request
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains(&format!("timed out after {}s", STOP_TIMEOUT_SECS)));
    }

    #[test]
    fn refresh_run_statuses_updates_terminal_states_and_failure_details() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();

        write_run(
            home,
            RunInstance {
                run_id: "run-succeeded".to_string(),
                dora_uuid: Some("uuid-succeeded".to_string()),
                dataflow_name: "demo".to_string(),
                dataflow_hash: "sha256:test".to_string(),
                started_at: "2026-03-09T00:00:00Z".to_string(),
                outcome: build_outcome(RunStatus::Running, None, None, None),
                ..RunInstance::default()
            },
        );
        write_runtime_log(home, "run-succeeded", "uuid-succeeded", "worker", "ok");

        write_run(
            home,
            RunInstance {
                run_id: "run-failed".to_string(),
                dora_uuid: Some("uuid-failed".to_string()),
                dataflow_name: "demo".to_string(),
                dataflow_hash: "sha256:test".to_string(),
                started_at: "2026-03-09T00:00:01Z".to_string(),
                outcome: build_outcome(RunStatus::Running, None, None, None),
                ..RunInstance::default()
            },
        );
        write_runtime_log(
            home,
            "run-failed",
            "uuid-failed",
            "worker",
            "AssertionError: expected x got y",
        );

        write_run(
            home,
            RunInstance {
                run_id: "run-runtime-stopped".to_string(),
                dora_uuid: Some("uuid-stopped".to_string()),
                dataflow_name: "demo".to_string(),
                dataflow_hash: "sha256:test".to_string(),
                started_at: "2026-03-09T00:00:02Z".to_string(),
                outcome: build_outcome(RunStatus::Running, None, None, None),
                ..RunInstance::default()
            },
        );

        write_run(
            home,
            RunInstance {
                run_id: "run-runtime-lost".to_string(),
                dora_uuid: Some("uuid-lost".to_string()),
                dataflow_name: "demo".to_string(),
                dataflow_hash: "sha256:test".to_string(),
                started_at: "2026-03-09T00:00:03Z".to_string(),
                outcome: build_outcome(RunStatus::Running, None, None, None),
                ..RunInstance::default()
            },
        );

        let mut runs = repo::list_run_instances(home).unwrap();
        let backend = TestBackend {
            start_result: Ok((Some("unused".to_string()), "started".to_string())),
            stop_result: Ok(()),
            list_result: Ok(vec![
                RuntimeDataflow {
                    id: "uuid-succeeded".to_string(),
                    status: RunStatus::Succeeded,
                },
                RuntimeDataflow {
                    id: "uuid-failed".to_string(),
                    status: RunStatus::Failed,
                },
                RuntimeDataflow {
                    id: "uuid-stopped".to_string(),
                    status: RunStatus::Stopped,
                },
            ]),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        service_runtime::refresh_run_statuses_with_backend(home, &mut runs, &backend).unwrap();

        let succeeded = repo::load_run(home, "run-succeeded").unwrap();
        assert_eq!(succeeded.status, RunStatus::Succeeded);
        assert_eq!(
            succeeded.termination_reason,
            Some(TerminationReason::Completed)
        );
        assert_eq!(succeeded.node_count_observed, 1);

        let failed = repo::load_run(home, "run-failed").unwrap();
        assert_eq!(failed.status, RunStatus::Failed);
        assert_eq!(failed.failure_node.as_deref(), Some("worker"));
        assert!(failed
            .failure_message
            .as_deref()
            .unwrap_or_default()
            .contains("AssertionError:"));

        let runtime_stopped = repo::load_run(home, "run-runtime-stopped").unwrap();
        assert_eq!(runtime_stopped.status, RunStatus::Stopped);
        assert_eq!(
            runtime_stopped.termination_reason,
            Some(TerminationReason::RuntimeStopped)
        );
        assert_eq!(runtime_stopped.stop_request.requested_at, None);

        let runtime_lost = repo::load_run(home, "run-runtime-lost").unwrap();
        assert_eq!(runtime_lost.status, RunStatus::Stopped);
        assert_eq!(
            runtime_lost.termination_reason,
            Some(TerminationReason::RuntimeLost)
        );
        assert_eq!(runtime_lost.failure_reason.as_deref(), Some("runtime_lost"));
    }

    #[test]
    fn refresh_run_statuses_preserves_user_stop_intent_across_runtime_terminal_states() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();

        write_run(
            home,
            RunInstance {
                run_id: "run-stop-succeeded".to_string(),
                dora_uuid: Some("uuid-stop-succeeded".to_string()),
                dataflow_name: "demo".to_string(),
                dataflow_hash: "sha256:test".to_string(),
                started_at: "2026-03-09T00:00:00Z".to_string(),
                stop_request: crate::runs::RunStopRequest {
                    requested_at: Some("2026-03-09T00:00:05Z".to_string()),
                    last_error: Some("dora stop timed out after 15s".to_string()),
                },
                outcome: build_outcome(RunStatus::Running, None, None, None),
                ..RunInstance::default()
            },
        );

        write_run(
            home,
            RunInstance {
                run_id: "run-stop-gone".to_string(),
                dora_uuid: Some("uuid-stop-gone".to_string()),
                dataflow_name: "demo".to_string(),
                dataflow_hash: "sha256:test".to_string(),
                started_at: "2026-03-09T00:00:01Z".to_string(),
                stop_request: crate::runs::RunStopRequest {
                    requested_at: Some("2026-03-09T00:00:06Z".to_string()),
                    last_error: None,
                },
                outcome: build_outcome(RunStatus::Running, None, None, None),
                ..RunInstance::default()
            },
        );

        let mut runs = repo::list_run_instances(home).unwrap();
        let backend = TestBackend {
            start_result: Ok((Some("unused".to_string()), "started".to_string())),
            stop_result: Ok(()),
            list_result: Ok(vec![RuntimeDataflow {
                id: "uuid-stop-succeeded".to_string(),
                status: RunStatus::Succeeded,
            }]),
            stop_calls: Arc::new(Mutex::new(Vec::new())),
        };

        service_runtime::refresh_run_statuses_with_backend(home, &mut runs, &backend).unwrap();

        let stop_succeeded = repo::load_run(home, "run-stop-succeeded").unwrap();
        assert_eq!(stop_succeeded.status, RunStatus::Stopped);
        assert_eq!(
            stop_succeeded.termination_reason,
            Some(TerminationReason::StoppedByUser)
        );
        assert_eq!(stop_succeeded.stop_request.requested_at, None);
        assert_eq!(stop_succeeded.stop_request.last_error, None);

        let stop_gone = repo::load_run(home, "run-stop-gone").unwrap();
        assert_eq!(stop_gone.status, RunStatus::Stopped);
        assert_eq!(
            stop_gone.termination_reason,
            Some(TerminationReason::StoppedByUser)
        );
        assert_eq!(stop_gone.failure_reason, None);
        assert_eq!(stop_gone.failure_message, None);
        assert_eq!(stop_gone.stop_request.requested_at, None);
    }

    #[tokio::test]
    async fn start_run_with_restart_strategy_stops_existing_active_run_first() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        setup_managed_node(home, "test-node", ".venv/bin/test-node");

        write_run(
            home,
            RunInstance {
                run_id: "old-run".to_string(),
                dora_uuid: Some("uuid-old".to_string()),
                dataflow_name: "demo".to_string(),
                dataflow_hash: "sha256:old".to_string(),
                started_at: "2026-03-09T00:00:00Z".to_string(),
                outcome: build_outcome(RunStatus::Running, None, None, None),
                ..RunInstance::default()
            },
        );

        let stop_calls = Arc::new(Mutex::new(Vec::new()));
        let backend = TestBackend {
            start_result: Ok((Some("uuid-new".to_string()), "started".to_string())),
            stop_result: Ok(()),
            list_result: Ok(vec![RuntimeDataflow {
                id: "uuid-old".to_string(),
                status: RunStatus::Running,
            }]),
            stop_calls: Arc::clone(&stop_calls),
        };

        let started = service_start::start_run_from_yaml_with_source_and_strategy_and_backend(
            home,
            "nodes:\n  - id: n1\n    node: test-node\n",
            "demo",
            None,
            RunSource::Cli,
            StartConflictStrategy::StopAndRestart,
            &backend,
        )
        .await
        .unwrap();

        assert_eq!(started.run.dora_uuid.as_deref(), Some("uuid-new"));
        assert_eq!(
            stop_calls.lock().unwrap().as_slice(),
            &["uuid-old".to_string()]
        );

        let old_run = repo::load_run(home, "old-run").unwrap();
        assert_eq!(old_run.status, RunStatus::Stopped);
        assert_eq!(
            old_run.termination_reason,
            Some(TerminationReason::StoppedByUser)
        );
    }
}
