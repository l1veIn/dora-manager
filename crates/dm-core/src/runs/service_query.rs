use std::path::Path;

use anyhow::Result;

use crate::runs::model::{
    PaginatedRuns, RunDetail, RunInstance, RunListFilter, RunLogChunk, RunSummary,
};
use crate::runs::repo;

pub fn list_runs(home: &Path, limit: i64, offset: i64) -> Result<PaginatedRuns> {
    list_runs_filtered(home, limit, offset, &RunListFilter::default())
}

pub fn list_runs_filtered(
    home: &Path,
    limit: i64,
    offset: i64,
    filter: &RunListFilter,
) -> Result<PaginatedRuns> {
    let runs = super::service_runtime::refresh_run_statuses(home)?;
    let runs = apply_run_list_filter(runs, filter);
    let total = runs.len() as i64;
    let offset = offset.max(0) as usize;
    let limit = limit.max(1) as usize;

    Ok(PaginatedRuns {
        runs: runs
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(to_summary)
            .collect(),
        total,
        limit: limit as i64,
        offset: offset as i64,
    })
}

pub fn get_active_run(home: &Path) -> Result<Option<RunInstance>> {
    Ok(list_active_runs(home)?.into_iter().next())
}

pub fn list_active_runs(home: &Path) -> Result<Vec<RunInstance>> {
    Ok(super::service_runtime::refresh_run_statuses(home)?
        .into_iter()
        .filter(|run| run.status.is_running())
        .collect())
}

pub fn get_run(home: &Path, run_id: &str) -> Result<RunDetail> {
    let _ = super::service_runtime::refresh_run_statuses(home)?;
    let mut run = repo::load_run(home, run_id)?;
    super::service_runtime::sync_run_outputs(home, &mut run)?;
    repo::save_run(home, &run)?;

    Ok(RunDetail {
        summary: to_summary(run.clone()),
        nodes: repo::list_run_nodes(home, &run.run_id)?,
    })
}

pub fn read_run_log(home: &Path, run_id: &str, node_id: &str) -> Result<String> {
    let path = repo::run_logs_dir(home, run_id).join(format!("{}.log", node_id));
    if !path.exists() {
        let mut run = repo::load_run(home, run_id)?;
        super::service_runtime::sync_run_outputs(home, &mut run)?;
        repo::save_run(home, &run)?;
    }
    repo::read_run_log_file(home, run_id, node_id)
}

pub fn read_run_transpiled(home: &Path, run_id: &str) -> Result<String> {
    repo::read_run_transpiled(home, run_id)
}

pub fn read_run_view(home: &Path, run_id: &str) -> Result<String> {
    repo::read_run_view(home, run_id)
}

pub fn read_run_log_chunk(
    home: &Path,
    run_id: &str,
    node_id: &str,
    offset: u64,
) -> Result<RunLogChunk> {
    let _ = super::service_runtime::refresh_run_statuses(home)?;
    let mut run = repo::load_run(home, run_id)?;
    super::service_runtime::sync_run_outputs(home, &mut run)?;
    repo::save_run(home, &run)?;

    let log_path = repo::run_logs_dir(home, run_id).join(format!("{}.log", node_id));
    let (content, next_offset) = if log_path.exists() {
        repo::read_run_log_chunk(home, run_id, node_id, offset)?
    } else {
        (String::new(), 0)
    };

    Ok(RunLogChunk {
        run_id: run_id.to_string(),
        node_id: node_id.to_string(),
        offset,
        next_offset,
        content,
        finished: !run.status.is_running(),
        status: run.status.as_str().to_string(),
    })
}

fn to_summary(run: RunInstance) -> RunSummary {
    RunSummary {
        id: run.run_id,
        name: run.dataflow_name,
        started_at: run.started_at,
        finished_at: run.stopped_at,
        exit_code: run.exit_code,
        source: run.source.as_str().to_string(),
        node_count: run.node_count_observed,
        status: run.status.as_str().to_string(),
        termination_reason: run
            .termination_reason
            .map(|reason| reason.as_str().to_string()),
        outcome_summary: run.outcome.summary,
        dora_uuid: run.dora_uuid,
        stop_requested_at: run.stop_request.requested_at,
        stop_request_error: run.stop_request.last_error,
        metrics: None,
    }
}

fn apply_run_list_filter(runs: Vec<RunInstance>, filter: &RunListFilter) -> Vec<RunInstance> {
    let normalized_status = filter
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());
    let normalized_search = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());

    runs.into_iter()
        .filter(|run| {
            if let Some(status) = normalized_status.as_deref() {
                if run.status.as_str() != status {
                    return false;
                }
            }

            if let Some(search) = normalized_search.as_deref() {
                let run_id = run.run_id.to_ascii_lowercase();
                let dataflow_name = run.dataflow_name.to_ascii_lowercase();
                if !run_id.contains(search) && !dataflow_name.contains(search) {
                    return false;
                }
            }

            true
        })
        .collect()
}
