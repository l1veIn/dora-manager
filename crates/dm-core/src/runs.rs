use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::events::{EventFilter, EventStore};

// ─── Models ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: String,
    pub name: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub node_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunNode {
    pub id: String,
    pub log_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDetail {
    #[serde(flatten)]
    pub summary: RunSummary,
    pub nodes: Vec<RunNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedRuns {
    pub runs: Vec<RunSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ─── API ───

/// List dataflow execution runs with pagination.
///
/// Queries events.db for `dataflow.start` events, then enriches each with
/// finish status from `dataflow.finish` events sharing the same case_id.
pub fn list_runs(home: &Path, limit: i64, offset: i64) -> Result<PaginatedRuns> {
    let store = EventStore::open(home)?;

    // Count total start events
    let total = store.count(&EventFilter {
        source: Some("dataflow".into()),
        activity: Some("dataflow.start".into()),
        ..Default::default()
    })?;

    // Fetch start events (paginated)
    let start_events = store.query(&EventFilter {
        source: Some("dataflow".into()),
        activity: Some("dataflow.start".into()),
        limit: Some(limit),
        offset: Some(offset),
        ..Default::default()
    })?;

    let mut runs = Vec::new();
    for ev in &start_events {
        let case_id = &ev.case_id;

        // Find corresponding finish event
        let finish_events = store.query(&EventFilter {
            source: Some("dataflow".into()),
            activity: Some("dataflow.finish".into()),
            case_id: Some(case_id.clone()),
            limit: Some(1),
            ..Default::default()
        })?;

        let (finished_at, exit_code, node_count) = if let Some(fin) = finish_events.first() {
            let attrs: serde_json::Value = fin
                .attributes
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::json!({}));
            (
                Some(fin.timestamp.clone()),
                attrs
                    .get("exit_code")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                attrs
                    .get("node_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            )
        } else {
            (None, None, 0)
        };

        // Extract name from attributes
        let attrs: serde_json::Value = ev
            .attributes
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::json!({}));
        let name = attrs
            .get("dataflow_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        runs.push(RunSummary {
            id: case_id.clone(),
            name,
            started_at: ev.timestamp.clone(),
            finished_at,
            exit_code,
            node_count,
        });
    }

    Ok(PaginatedRuns {
        runs,
        total,
        limit,
        offset,
    })
}

/// Get detailed info for a single run.
///
/// Combines events.db metadata with the `out/<df_id>/` log file directory.
pub fn get_run(home: &Path, run_id: &str) -> Result<RunDetail> {
    let store = EventStore::open(home)?;

    // Get start event
    let start_events = store.query(&EventFilter {
        source: Some("dataflow".into()),
        activity: Some("dataflow.start".into()),
        case_id: Some(run_id.into()),
        limit: Some(1),
        ..Default::default()
    })?;

    let start_ev = start_events
        .first()
        .ok_or_else(|| anyhow::anyhow!("Run '{}' not found", run_id))?;

    let attrs: serde_json::Value = start_ev
        .attributes
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));
    let name = attrs
        .get("dataflow_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Get finish event
    let finish_events = store.query(&EventFilter {
        source: Some("dataflow".into()),
        activity: Some("dataflow.finish".into()),
        case_id: Some(run_id.into()),
        limit: Some(1),
        ..Default::default()
    })?;

    let (finished_at, exit_code, node_count_from_event) = if let Some(fin) = finish_events.first() {
        let fa: serde_json::Value = fin
            .attributes
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::json!({}));
        (
            Some(fin.timestamp.clone()),
            fa.get("exit_code")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            fa.get("node_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        )
    } else {
        (None, None, 0)
    };

    // Scan out/<run_id>/ for node log files
    let out_dir = home.join("run").join("out").join(run_id);
    let mut nodes = Vec::new();
    if out_dir.exists() {
        if let Ok(entries) = fs::read_dir(&out_dir) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().to_string();
                if let Some(node_id) = filename
                    .strip_prefix("log_")
                    .and_then(|s| s.strip_suffix(".txt"))
                {
                    let log_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    nodes.push(RunNode {
                        id: node_id.to_string(),
                        log_size,
                    });
                }
            }
        }
    }
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    let node_count = if nodes.is_empty() {
        node_count_from_event
    } else {
        nodes.len() as u32
    };

    Ok(RunDetail {
        summary: RunSummary {
            id: run_id.to_string(),
            name,
            started_at: start_ev.timestamp.clone(),
            finished_at,
            exit_code,
            node_count,
        },
        nodes,
    })
}

/// Read the log content for a specific node in a run.
pub fn get_run_logs(home: &Path, run_id: &str, node_id: &str) -> Result<String> {
    let log_path = home
        .join("run")
        .join("out")
        .join(run_id)
        .join(format!("log_{}.txt", node_id));

    fs::read_to_string(&log_path)
        .with_context(|| format!("Log for node '{}' in run '{}' not found", node_id, run_id))
}

/// Delete a run's log directory and events.
pub fn delete_run(home: &Path, run_id: &str) -> Result<()> {
    // Delete out/<run_id>/ directory
    let out_dir = home.join("run").join("out").join(run_id);
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)
            .with_context(|| format!("Failed to delete run directory for '{}'", run_id))?;
    }

    // Delete events with this case_id
    let store = EventStore::open(home)?;
    store.delete_by_case_id(run_id)?;

    Ok(())
}

/// Clean old runs, keeping only the most recent `keep` entries.
/// Returns the number of runs deleted.
pub fn clean_runs(home: &Path, keep: usize) -> Result<u32> {
    let all = list_runs(home, 10000, 0)?;
    let mut deleted = 0u32;

    if all.runs.len() > keep {
        for run in &all.runs[keep..] {
            if let Err(e) = delete_run(home, &run.id) {
                eprintln!("Warning: failed to clean run {}: {}", run.id, e);
            } else {
                deleted += 1;
            }
        }
    }

    Ok(deleted)
}
