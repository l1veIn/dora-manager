use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::{Command as StdCommand, Stdio};

use anyhow::{bail, Context, Result};

use crate::dora;
use crate::runs::model::RunStatus;

type StartResult = (Option<String>, String);
type BoxFutureResult<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct RuntimeDataflow {
    pub id: String,
    pub status: RunStatus,
}

pub trait RuntimeBackend {
    fn start_detached<'a>(
        &'a self,
        home: &'a Path,
        transpiled_path: &'a Path,
    ) -> BoxFutureResult<'a, StartResult>;

    fn stop<'a>(&'a self, home: &'a Path, dora_uuid: &'a str) -> BoxFutureResult<'a, ()>;

    fn list(&self, home: &Path) -> Result<Vec<RuntimeDataflow>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DoraCliBackend;

pub fn default_backend() -> DoraCliBackend {
    DoraCliBackend
}

impl RuntimeBackend for DoraCliBackend {
    fn start_detached<'a>(
        &'a self,
        home: &'a Path,
        transpiled_path: &'a Path,
    ) -> BoxFutureResult<'a, StartResult> {
        Box::pin(async move {
            let dora_bin = dora::active_dora_bin(home)?;
            let output = tokio::process::Command::new(&dora_bin)
                .args(["start", &transpiled_path.to_string_lossy(), "--detach"])
                .output()
                .await
                .with_context(|| format!("Failed to run dora at {}", dora_bin.display()))?;

            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if !output.status.success() {
                let message = if stderr.is_empty() { stdout } else { stderr };
                bail!(message);
            }

            let combined = if stderr.is_empty() {
                stdout.clone()
            } else if stdout.is_empty() {
                stderr.clone()
            } else {
                format!("{}\n{}", stdout, stderr)
            };

            Ok((
                extract_dataflow_id(&combined),
                if stdout.is_empty() { combined } else { stdout },
            ))
        })
    }

    fn stop<'a>(&'a self, home: &'a Path, dora_uuid: &'a str) -> BoxFutureResult<'a, ()> {
        Box::pin(async move {
            let args = vec!["stop".to_string(), dora_uuid.to_string()];
            let (code, stdout, stderr) = dora::run_dora(home, &args, false).await?;
            if code != 0 {
                let message = if stderr.trim().is_empty() {
                    stdout
                } else {
                    stderr
                };
                bail!(message.trim().to_string());
            }
            Ok(())
        })
    }

    fn list(&self, home: &Path) -> Result<Vec<RuntimeDataflow>> {
        let dora_bin = dora::active_dora_bin(home)?;
        let output = StdCommand::new(&dora_bin)
            .arg("list")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to run dora at {}", dora_bin.display()))?;

        if !output.status.success() {
            bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }

        Ok(parse_runtime_dataflows(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }
}

pub fn extract_dataflow_id(output: &str) -> Option<String> {
    const PREFIXES: &[&str] = &["dataflow start triggered: ", "dataflow started: "];

    output.lines().find_map(|line| {
        PREFIXES.iter().find_map(|prefix| {
            line.strip_prefix(prefix)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
    })
}

fn parse_runtime_dataflows(stdout: &str) -> Vec<RuntimeDataflow> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("UUID"))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let id = parts.first()?.to_string();
            if uuid::Uuid::parse_str(&id).is_err() {
                return None;
            }

            Some(RuntimeDataflow {
                id,
                status: map_status(parts.get(2).copied()),
            })
        })
        .collect()
}

fn map_status(value: Option<&str>) -> RunStatus {
    match value {
        Some("Running") | Some("running") => RunStatus::Running,
        Some("Succeeded") | Some("succeeded") => RunStatus::Succeeded,
        Some("Stopped") | Some("stopped") => RunStatus::Stopped,
        Some("Failed") | Some("failed") => RunStatus::Failed,
        _ => RunStatus::Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_dataflow_id, parse_runtime_dataflows};
    use crate::runs::model::RunStatus;

    #[test]
    fn extracts_dataflow_id_from_start_messages() {
        let output = "booting\n\
dataflow start triggered: 019cc181-adad-7654-aa78-63502362337b\n\
done";
        assert_eq!(
            extract_dataflow_id(output).as_deref(),
            Some("019cc181-adad-7654-aa78-63502362337b")
        );

        let output = "dataflow started: 019cc181-adad-7654-aa78-63502362337b";
        assert_eq!(
            extract_dataflow_id(output).as_deref(),
            Some("019cc181-adad-7654-aa78-63502362337b")
        );
    }

    #[test]
    fn ignores_non_matching_start_output() {
        assert_eq!(extract_dataflow_id("started demo"), None);
        assert_eq!(extract_dataflow_id(""), None);
    }

    #[test]
    fn parses_runtime_table_rows() {
        let stdout = "\
UUID Name Status Nodes CPU Memory
019cc181-adad-7654-aa78-63502362337b qwen-dev Running 7 0.0% 0.0 GB
019cc181-adad-7654-aa78-635023623380 qwen-dev Succeeded 7 0.0% 0.0 GB
019cc181-adad-7654-aa78-635023623381 qwen-dev Failed 7 0.0% 0.0 GB
";

        let rows = parse_runtime_dataflows(stdout);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].status, RunStatus::Running);
        assert_eq!(rows[1].status, RunStatus::Succeeded);
        assert_eq!(rows[2].status, RunStatus::Failed);
    }

    #[test]
    fn skips_noise_and_defaults_unknown_status_to_stopped() {
        let stdout = "\
flow-a
UUID Name Status Nodes CPU Memory
not-a-uuid demo Running 1 0.0% 0.0 GB
019cc181-adad-7654-aa78-63502362337b demo Mystery 1 0.0% 0.0 GB
";

        let rows = parse_runtime_dataflows(stdout);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "019cc181-adad-7654-aa78-63502362337b");
        assert_eq!(rows[0].status, RunStatus::Stopped);
    }
}
