use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::dora;
use crate::runs::model::{NodeMetrics, RunMetrics};
use crate::runs::repo;

/// Collect runtime metrics for a single running dataflow.
///
/// Returns `Ok(None)` when the run is not currently active in the Dora runtime.
pub fn get_run_metrics(home: &Path, run_id: &str) -> Result<Option<RunMetrics>> {
    let run = repo::load_run(home, run_id)?;
    if !run.status.is_running() {
        return Ok(None);
    }
    let dora_uuid = match run.dora_uuid.as_deref() {
        Some(uuid) => uuid,
        None => return Ok(None),
    };

    let dataflow_metrics = collect_dataflow_metrics(home)?;
    let df = dataflow_metrics.get(dora_uuid);

    let node_metrics = collect_node_metrics(home, dora_uuid)?;

    Ok(Some(RunMetrics {
        cpu: df.and_then(|d| d.cpu),
        memory_mb: df.and_then(|d| d.memory_mb),
        nodes: node_metrics,
    }))
}

/// Collect metrics for all active dataflows in one shot.
/// Returns a map keyed by dora_uuid.
pub fn collect_all_active_metrics(home: &Path) -> Result<HashMap<String, RunMetrics>> {
    let dataflow_map = collect_dataflow_metrics(home)?;
    let mut result: HashMap<String, RunMetrics> = HashMap::new();

    for (uuid, df) in &dataflow_map {
        let node_metrics = collect_node_metrics(home, uuid).unwrap_or_default();
        result.insert(
            uuid.clone(),
            RunMetrics {
                cpu: df.cpu,
                memory_mb: df.memory_mb,
                nodes: node_metrics,
            },
        );
    }

    Ok(result)
}

// ─── Internal helpers ───

struct DataflowAggregateMetrics {
    cpu: Option<f64>,
    memory_mb: Option<f64>,
}

fn collect_dataflow_metrics(home: &Path) -> Result<HashMap<String, DataflowAggregateMetrics>> {
    let dora_bin = dora::active_dora_bin(home)?;
    let output = Command::new(&dora_bin)
        .args(["list", "--format", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to run dora list at {}", dora_bin.display()))?;

    if !output.status.success() {
        return Ok(HashMap::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_dataflow_metrics_json(&stdout)
}

fn collect_node_metrics(home: &Path, dora_uuid: &str) -> Result<Vec<NodeMetrics>> {
    let dora_bin = dora::active_dora_bin(home)?;
    let output = Command::new(&dora_bin)
        .args(["node", "list", "--format", "json", "--dataflow", dora_uuid])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to run dora node list at {}", dora_bin.display()))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_node_metrics_json(&stdout))
}

/// Parse newline-delimited JSON from `dora list --format json`.
/// Each line: {"uuid":"...","name":"...","status":"Running","nodes":8,"cpu":23.16,"memory":1.83}
fn parse_dataflow_metrics_json(stdout: &str) -> Result<HashMap<String, DataflowAggregateMetrics>> {
    let mut map = HashMap::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(uuid) = v.get("uuid").and_then(|v| v.as_str()) else {
            continue;
        };
        let cpu = v.get("cpu").and_then(|v| v.as_f64());
        // Memory from dora list is in GB (e.g. 1.83). Convert to MB.
        let memory_mb = v
            .get("memory")
            .and_then(|v| v.as_f64())
            .map(|gb| gb * 1024.0);
        map.insert(
            uuid.to_string(),
            DataflowAggregateMetrics { cpu, memory_mb },
        );
    }
    Ok(map)
}

/// Parse newline-delimited JSON from `dora node list --format json`.
/// Each line: {"node":"x","status":"Running","pid":"123","cpu":"0.0%","memory":"54 MB","dataflow":"name"}
fn parse_node_metrics_json(stdout: &str) -> Vec<NodeMetrics> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            let id = v.get("node")?.as_str()?.to_string();
            let status = v
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let pid = v.get("pid").and_then(|v| v.as_str()).map(str::to_string);
            let cpu = v.get("cpu").and_then(|v| v.as_str()).map(str::to_string);
            let memory = v.get("memory").and_then(|v| v.as_str()).map(str::to_string);
            Some(NodeMetrics {
                id,
                status,
                pid,
                cpu,
                memory,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dataflow_metrics_json_extracts_values() {
        let input = r#"{"uuid":"019cd2f0-60ce-7e6c-876f-5d42ba350287","name":"bunny","status":"Running","nodes":8,"cpu":23.16,"memory":1.83}"#;
        let map = parse_dataflow_metrics_json(input).unwrap();
        let m = map
            .get("019cd2f0-60ce-7e6c-876f-5d42ba350287")
            .expect("should find uuid");
        assert!((m.cpu.unwrap() - 23.16).abs() < 0.01);
        assert!((m.memory_mb.unwrap() - 1874.0).abs() < 1.0); // 1.83 * 1024
    }

    #[test]
    fn parse_node_metrics_json_extracts_all_nodes() {
        let input = r#"{"node":"dora-qwen","status":"Running","pid":"67842","cpu":"0.0%","memory":"1143 MB","dataflow":"bunny"}
{"node":"dora-vad","status":"Running","pid":"67843","cpu":"23.7%","memory":"85 MB","dataflow":"bunny"}"#;
        let nodes = parse_node_metrics_json(input);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, "dora-qwen");
        assert_eq!(nodes[0].memory.as_deref(), Some("1143 MB"));
        assert_eq!(nodes[1].cpu.as_deref(), Some("23.7%"));
    }

    #[test]
    fn parse_node_metrics_json_handles_empty_input() {
        assert!(parse_node_metrics_json("").is_empty());
        assert!(parse_node_metrics_json("  \n  ").is_empty());
    }

    #[test]
    fn parse_dataflow_metrics_json_handles_malformed_lines() {
        let input = "not json\n{\"uuid\":\"abc\"}\n";
        let map = parse_dataflow_metrics_json(input).unwrap();
        // Second line has uuid but no cpu/memory
        let m = map.get("abc").expect("should find uuid");
        assert!(m.cpu.is_none());
        assert!(m.memory_mb.is_none());
    }
}
