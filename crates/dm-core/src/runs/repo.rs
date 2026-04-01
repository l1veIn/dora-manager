use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::model::RunInstance;

pub fn runs_dir(home: &Path) -> PathBuf {
    home.join("runs")
}

pub fn run_dir(home: &Path, run_id: &str) -> PathBuf {
    runs_dir(home).join(run_id)
}

pub fn run_json_path(home: &Path, run_id: &str) -> PathBuf {
    run_dir(home, run_id).join("run.json")
}

pub fn run_snapshot_path(home: &Path, run_id: &str) -> PathBuf {
    run_dir(home, run_id).join("dataflow.yml")
}

pub fn run_view_json_path(home: &Path, run_id: &str) -> PathBuf {
    run_dir(home, run_id).join("view.json")
}

pub fn run_transpiled_path(home: &Path, run_id: &str) -> PathBuf {
    run_dir(home, run_id).join("dataflow.transpiled.yml")
}

pub fn run_logs_dir(home: &Path, run_id: &str) -> PathBuf {
    run_dir(home, run_id).join("logs")
}

pub fn run_out_dir(home: &Path, run_id: &str) -> PathBuf {
    run_dir(home, run_id).join("out")
}

pub fn create_layout(home: &Path, run_id: &str) -> Result<PathBuf> {
    let dir = run_dir(home, run_id);
    fs::create_dir_all(run_logs_dir(home, run_id))
        .with_context(|| format!("Failed to create logs dir for run '{}'", run_id))?;
    fs::create_dir_all(run_out_dir(home, run_id))
        .with_context(|| format!("Failed to create output dir for run '{}'", run_id))?;
    Ok(dir)
}

pub fn load_run(home: &Path, run_id: &str) -> Result<RunInstance> {
    let path = run_json_path(home, run_id);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read run metadata {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse run metadata {}", path.display()))
}

pub fn save_run(home: &Path, run: &RunInstance) -> Result<()> {
    let path = run_json_path(home, &run.run_id);
    let content = serde_json::to_string_pretty(run)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write run metadata {}", path.display()))
}

pub fn list_run_instances(home: &Path) -> Result<Vec<RunInstance>> {
    let base = runs_dir(home);
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    for entry in fs::read_dir(&base).context("Failed to read runs directory")? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }

        let run_id = entry.file_name().to_string_lossy().to_string();
        if let Ok(run) = load_run(home, &run_id) {
            runs.push(run);
        }
    }

    runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(runs)
}

pub fn read_run_dataflow(home: &Path, run_id: &str) -> Result<String> {
    let path = run_snapshot_path(home, run_id);
    fs::read_to_string(&path)
        .with_context(|| format!("Failed to read run snapshot {}", path.display()))
}

pub fn read_run_transpiled(home: &Path, run_id: &str) -> Result<String> {
    let path = run_transpiled_path(home, run_id);
    fs::read_to_string(&path)
        .with_context(|| format!("Failed to read transpiled run snapshot {}", path.display()))
}

pub fn read_run_view(home: &Path, run_id: &str) -> Result<String> {
    let path = run_view_json_path(home, run_id);
    fs::read_to_string(&path)
        .with_context(|| format!("Failed to read run view.json {}", path.display()))
}

pub fn read_run_log_file(home: &Path, run_id: &str, node_id: &str) -> Result<String> {
    let path = run_logs_dir(home, run_id).join(format!("{}.log", node_id));
    fs::read_to_string(&path).with_context(|| format!("Failed to read node log {}", path.display()))
}

pub fn read_run_log_chunk(
    home: &Path,
    run_id: &str,
    node_id: &str,
    offset: u64,
) -> Result<(String, u64)> {
    let path = run_logs_dir(home, run_id).join(format!("{}.log", node_id));
    let mut file = fs::File::open(&path)
        .with_context(|| format!("Failed to read node log {}", path.display()))?;
    let len = file
        .metadata()
        .with_context(|| format!("Failed to read node log metadata {}", path.display()))?
        .len();
    let start = offset.min(len);
    file.seek(SeekFrom::Start(start))
        .with_context(|| format!("Failed to seek node log {}", path.display()))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .with_context(|| format!("Failed to read node log {}", path.display()))?;
    Ok((String::from_utf8_lossy(&buf).to_string(), len))
}

pub fn list_run_nodes(home: &Path, run_id: &str) -> Result<Vec<super::model::RunNode>> {
    let logs_dir = run_logs_dir(home, run_id);
    let mut nodes = Vec::new();
    if !logs_dir.exists() {
        return Ok(nodes);
    }

    for entry in fs::read_dir(&logs_dir)
        .with_context(|| format!("Failed to read logs directory {}", logs_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(node_id) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_string())
        else {
            continue;
        };
        let log_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        nodes.push(super::model::RunNode {
            id: node_id,
            log_size,
        });
    }

    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(nodes)
}

pub fn delete_run(home: &Path, run_id: &str) -> Result<()> {
    let dir = run_dir(home, run_id);
    if dir.exists() {
        fs::remove_dir_all(&dir)
            .with_context(|| format!("Failed to delete run directory {}", dir.display()))?;
    }
    Ok(())
}
