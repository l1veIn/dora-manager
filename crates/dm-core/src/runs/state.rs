use std::path::Path;

use chrono::Utc;

use super::model::{RunInstance, RunOutcome, RunStatus, TerminationReason};
use super::repo;

pub(crate) fn parse_failure_details(message: &str) -> (Option<String>, String) {
    let trimmed = message.trim().to_string();
    let lower = trimmed.to_lowercase();
    let needle = "node ";
    if let Some(start) = lower.find(needle) {
        let rest = &trimmed[start + needle.len()..];
        if let Some((node, detail)) = rest.split_once(" failed:") {
            return (
                Some(node.trim().to_string()),
                detail.trim_start_matches(':').trim().to_string(),
            );
        }
    }
    (None, trimmed)
}

pub(crate) fn infer_failure_details(
    home: &Path,
    run: &RunInstance,
) -> Option<(Option<String>, Option<String>)> {
    if run.failure_node.is_some() && run.failure_message.is_some() {
        return Some((run.failure_node.clone(), run.failure_message.clone()));
    }

    for node_id in &run.nodes_observed {
        let Ok(log) = repo::read_run_log_file(home, &run.run_id, node_id) else {
            continue;
        };
        let Some(summary) = extract_error_summary(&log) else {
            continue;
        };
        return Some((Some(node_id.clone()), Some(summary)));
    }

    if run.failure_node.is_some() || run.failure_message.is_some() {
        return Some((run.failure_node.clone(), run.failure_message.clone()));
    }

    None
}

pub(crate) fn apply_terminal_state(
    run: &mut RunInstance,
    status: RunStatus,
    termination_reason: Option<TerminationReason>,
    exit_code: Option<i32>,
    failure_reason: Option<String>,
    failure_node: Option<String>,
    failure_message: Option<String>,
    observed_at: Option<String>,
) {
    let summary = build_outcome(
        status,
        termination_reason,
        failure_node.clone(),
        failure_message.clone(),
    );

    run.status = status;
    run.termination_reason = termination_reason;
    run.exit_code = exit_code;
    run.failure_reason = failure_reason;
    run.failure_node = failure_node;
    run.failure_message = failure_message;
    if run.stopped_at.is_none() {
        run.stopped_at = Some(Utc::now().to_rfc3339());
    }
    if observed_at.is_some() {
        run.runtime_observed_at = observed_at;
    }
    run.outcome = summary;
}

pub(crate) fn build_outcome(
    status: RunStatus,
    termination_reason: Option<TerminationReason>,
    failure_node: Option<String>,
    failure_message: Option<String>,
) -> RunOutcome {
    let summary = match status {
        RunStatus::Running => "Running".to_string(),
        RunStatus::Succeeded => "Succeeded".to_string(),
        RunStatus::Stopped => match termination_reason {
            Some(TerminationReason::StoppedByUser) => "Stopped by user".to_string(),
            Some(TerminationReason::RuntimeLost) => {
                "Stopped after Dora runtime lost track of the dataflow".to_string()
            }
            Some(TerminationReason::RuntimeStopped) => "Stopped by Dora runtime".to_string(),
            _ => "Stopped".to_string(),
        },
        RunStatus::Failed => match (failure_node, failure_message) {
            (Some(node), Some(message)) if !message.is_empty() => {
                format!("Failed: {} {}", node, message)
            }
            (Some(node), _) => format!("Failed: {}", node),
            (_, Some(message)) if !message.is_empty() => format!("Failed: {}", message),
            _ => "Failed".to_string(),
        },
    };

    RunOutcome {
        status,
        termination_reason,
        summary,
    }
}

fn extract_error_summary(log: &str) -> Option<String> {
    let trimmed = log.trim();
    if trimmed.is_empty() {
        return None;
    }

    for needle in [
        "AssertionError:",
        "thread 'main' panicked at",
        "panic:",
        "ERROR",
    ] {
        if let Some(idx) = trimmed.find(needle) {
            return Some(compact_error_text(&trimmed[idx..]));
        }
    }

    if trimmed.contains("Traceback (most recent call last):") {
        if let Some(line) = trimmed.lines().rev().find(|line| !line.trim().is_empty()) {
            return Some(compact_error_text(line));
        }
    }

    None
}

fn compact_error_text(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_LEN: usize = 240;
    if compact.len() <= MAX_LEN {
        compact
    } else {
        format!("{}...", &compact[..MAX_LEN])
    }
}

#[cfg(test)]
mod tests {
    use super::{compact_error_text, extract_error_summary, parse_failure_details};

    #[test]
    fn parse_failure_details_extracts_node_name() {
        let (node, message) = parse_failure_details(
            "Node fail_assert failed: node was killed by dora because it didn't react",
        );
        assert_eq!(node.as_deref(), Some("fail_assert"));
        assert_eq!(message, "node was killed by dora because it didn't react");
    }

    #[test]
    fn extract_error_summary_prefers_assertion_error() {
        let log = r#"
Traceback (most recent call last):
  File "main.py", line 1, in <module>
AssertionError: Expected [ "system-test-expected" ], got [ "system-test-actual" ]
"#;
        let summary = extract_error_summary(log).expect("summary");
        assert!(summary.starts_with("AssertionError:"));
        assert!(summary.contains("system-test-expected"));
        assert!(summary.contains("system-test-actual"));
    }

    #[test]
    fn compact_error_text_collapses_whitespace() {
        let compact = compact_error_text("AssertionError:\n  Expected   x\n  got y");
        assert_eq!(compact, "AssertionError: Expected x got y");
    }
}
