use crate::types::*;

#[test]
fn env_item_serialization() {
    let item = EnvItem {
        name: "Python".into(),
        found: true,
        path: Some("/usr/bin/python3".into()),
        version: Some("3.12.0".into()),
        suggestion: None,
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("\"found\":true"));
    assert!(json.contains("\"name\":\"Python\""));
    assert!(json.contains("/usr/bin/python3"));

    // Roundtrip
    let parsed: EnvItem = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "Python");
    assert!(parsed.found);
    assert_eq!(parsed.path.unwrap(), "/usr/bin/python3");
}

#[test]
fn env_item_not_found_serialization() {
    let item = EnvItem {
        name: "uv".into(),
        found: false,
        path: None,
        version: None,
        suggestion: Some("pip install uv".into()),
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("\"found\":false"));
    assert!(json.contains("\"path\":null"));
    assert!(json.contains("pip install uv"));
}

#[test]
fn doctor_report_serialization() {
    let report = DoctorReport {
        python: EnvItem {
            name: "Python".into(),
            found: true,
            path: Some("/usr/bin/python3".into()),
            version: Some("3.12".into()),
            suggestion: None,
        },
        uv: EnvItem {
            name: "uv".into(),
            found: false,
            path: None,
            version: None,
            suggestion: Some("install uv".into()),
        },
        rust: EnvItem {
            name: "Rust".into(),
            found: true,
            path: Some("/usr/bin/cargo".into()),
            version: Some("1.75".into()),
            suggestion: None,
        },
        installed_versions: vec![
            InstalledVersion {
                version: "0.4.1".into(),
                active: true,
            },
            InstalledVersion {
                version: "0.3.9".into(),
                active: false,
            },
        ],
        active_version: Some("0.4.1".into()),
        active_binary_ok: true,
        all_ok: false,
    };
    let json = serde_json::to_string_pretty(&report).unwrap();
    let parsed: DoctorReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.installed_versions.len(), 2);
    assert!(parsed.installed_versions[0].active);
    assert!(!parsed.installed_versions[1].active);
    assert_eq!(parsed.active_version, Some("0.4.1".into()));
    assert!(parsed.active_binary_ok);
    assert!(!parsed.all_ok); // uv not found
}

#[test]
fn versions_report_serialization() {
    let report = VersionsReport {
        installed: vec![InstalledVersion {
            version: "0.4.1".into(),
            active: true,
        }],
        available: vec![
            AvailableVersion {
                tag: "0.4.1".into(),
                installed: true,
            },
            AvailableVersion {
                tag: "0.4.0".into(),
                installed: false,
            },
        ],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: VersionsReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.installed.len(), 1);
    assert_eq!(parsed.available.len(), 2);
    assert!(parsed.available[0].installed);
    assert!(!parsed.available[1].installed);
}

#[test]
fn install_progress_serialization() {
    let progress = InstallProgress {
        phase: InstallPhase::Downloading {
            bytes_done: 5000,
            bytes_total: 10000,
        },
        message: "Downloading...".into(),
    };
    let json = serde_json::to_string(&progress).unwrap();
    assert!(json.contains("Downloading"));
    assert!(json.contains("5000"));
    assert!(json.contains("10000"));

    let parsed: InstallProgress = serde_json::from_str(&json).unwrap();
    match parsed.phase {
        InstallPhase::Downloading {
            bytes_done,
            bytes_total,
        } => {
            assert_eq!(bytes_done, 5000);
            assert_eq!(bytes_total, 10000);
        }
        _ => panic!("Expected Downloading phase"),
    }
}

#[test]
fn install_result_serialization() {
    let result = InstallResult {
        version: "0.4.1".into(),
        method: InstallMethod::Binary,
        set_active: true,
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: InstallResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.version, "0.4.1");
    assert!(parsed.set_active);
    match parsed.method {
        InstallMethod::Binary => {}
        _ => panic!("Expected Binary method"),
    }
}

#[test]
fn install_result_source_method() {
    let result = InstallResult {
        version: "0.3.9".into(),
        method: InstallMethod::Source,
        set_active: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: InstallResult = serde_json::from_str(&json).unwrap();
    match parsed.method {
        InstallMethod::Source => {}
        _ => panic!("Expected Source method"),
    }
}

#[test]
fn runtime_result_serialization() {
    let result = RuntimeResult {
        success: true,
        message: "Started".into(),
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: RuntimeResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.success);
    assert_eq!(parsed.message, "Started");
}

#[test]
fn status_report_serialization() {
    let report = StatusReport {
        active_version: Some("0.4.1".into()),
        actual_version: Some("0.4.1".into()),
        dm_home: "/home/user/.dm".into(),
        runtime_running: false,
        runtime_output: String::new(),
        active_runs: vec![StatusRunEntry {
            run_id: "5c49b211-5575-4a7e-a666-cf32c198ea5e".into(),
            dataflow_name: "flow1".into(),
            status: "running".into(),
            started_at: "2026-03-06T05:32:40+00:00".into(),
            finished_at: None,
            expected_nodes: 3,
            observed_nodes: 2,
            dora_uuid: Some("019cc181-adad-7654-aa78-63502362337b".into()),
            outcome_summary: "Running".into(),
        }],
        recent_runs: vec![StatusRunEntry {
            run_id: "ec7155dd-a357-4261-993b-72f18e304ea5".into(),
            dataflow_name: "flow2".into(),
            status: "stopped".into(),
            started_at: "2026-03-06T05:30:00+00:00".into(),
            finished_at: Some("2026-03-06T05:31:00+00:00".into()),
            expected_nodes: 4,
            observed_nodes: 4,
            dora_uuid: Some("019cc181-adad-7654-aa78-635023623380".into()),
            outcome_summary: "Stopped by user".into(),
        }],
        dora_probe: vec![RuntimeDataflowStatus {
            id: "019cc181-adad-7654-aa78-63502362337b".into(),
            dataflow_name: "flow1".into(),
            runtime_name: Some("endless-junco".into()),
            status: "Running".into(),
            expected_nodes: 3,
            observed_nodes: 2,
            cpu: Some("0.0%".into()),
            memory: Some("0.0".into()),
        }],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: StatusReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.active_runs.len(), 1);
    assert_eq!(parsed.recent_runs.len(), 1);
    assert_eq!(parsed.dora_probe.len(), 1);
    assert!(!parsed.runtime_running);
}

#[test]
fn setup_report_serialization() {
    let report = SetupReport {
        python_installed: true,
        uv_installed: false,
        dora_installed: true,
        dora_version: Some("0.4.1".into()),
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: SetupReport = serde_json::from_str(&json).unwrap();
    assert!(parsed.python_installed);
    assert!(!parsed.uv_installed);
    assert!(parsed.dora_installed);
    assert_eq!(parsed.dora_version, Some("0.4.1".into()));
}

#[test]
fn all_install_phases_serialize() {
    let phases = vec![
        InstallPhase::Fetching,
        InstallPhase::Downloading {
            bytes_done: 0,
            bytes_total: 1024,
        },
        InstallPhase::Extracting,
        InstallPhase::Building,
        InstallPhase::Done,
    ];
    for phase in phases {
        let progress = InstallProgress {
            phase,
            message: "test".into(),
        };
        let json = serde_json::to_string(&progress).unwrap();
        let _: InstallProgress = serde_json::from_str(&json).unwrap();
    }
}
