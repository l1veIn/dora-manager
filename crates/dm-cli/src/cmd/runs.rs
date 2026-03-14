use std::io::Write;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;

pub async fn list(home: &Path) -> Result<()> {
    let result = dm_core::runs::list_runs(home, 20, 0)?;
    if result.runs.is_empty() {
        println!("No dataflow runs recorded yet.");
    } else {
        println!(
            "{:<40} {:<15} {:<22} {:<6} Nodes",
            "ID", "Name", "Started", "Exit"
        );
        println!("{}", "─".repeat(90));
        for run in &result.runs {
            let status_icon = match run.status.as_str() {
                "running" => "⏳".to_string(),
                "succeeded" => "✅".to_string(),
                "stopped" => match run.exit_code {
                    Some(0) => "✅".to_string(),
                    None => "⏹".to_string(),
                    Some(c) => format!("❌ {}", c),
                },
                "failed" => match run.exit_code {
                    Some(c) => format!("❌ {}", c),
                    None => "❌".to_string(),
                },
                _ => "?".to_string(),
            };
            let started = &run.started_at[..19]; // trim timezone
            println!(
                "{:<40} {:<15} {:<22} {:<6} {}",
                run.id.dimmed(),
                run.name.bold(),
                started,
                status_icon,
                run.node_count
            );
        }
        println!("\nShowing {}/{} runs.", result.runs.len(), result.total);
    }
    Ok(())
}

pub async fn stop(home: &Path, run_id: String) -> Result<()> {
    let run = dm_core::runs::stop_run(home, &run_id).await?;
    println!("{} Stopped run {}", "✅".green(), run.run_id.bold());
    if let Some(stopped_at) = run.stopped_at {
        println!("  Stopped at: {}", stopped_at.dimmed());
    }
    Ok(())
}

pub fn delete(home: &Path, run_ids: Vec<String>) -> Result<()> {
    let total = run_ids.len();
    let mut deleted = 0usize;
    let mut failed = Vec::new();

    for run_id in run_ids {
        match dm_core::runs::delete_run(home, &run_id) {
            Ok(()) => {
                deleted += 1;
                println!("{} Deleted run {}", "✅".green(), run_id.bold());
            }
            Err(e) => {
                failed.push((run_id, e.to_string()));
            }
        }
    }

    if !failed.is_empty() {
        eprintln!(
            "{} Deleted {}/{} runs. Failures:",
            "⚠".yellow(),
            deleted,
            total
        );
        for (run_id, message) in failed {
            eprintln!("  {} {}", run_id.bold(), message);
        }
        anyhow::bail!("Failed to delete one or more runs");
    }
    Ok(())
}

pub async fn logs(
    home: &Path,
    run_id: String,
    node_id: Option<String>,
    follow: bool,
) -> Result<()> {
    if let Some(nid) = node_id {
        if follow {
            follow_run_log(home, &run_id, &nid).await?;
        } else {
            let content = dm_core::runs::read_run_log(home, &run_id, &nid)?;
            if content.is_empty() {
                println!("(empty log)");
            } else {
                print!("{}", content);
            }
        }
    } else if follow {
        anyhow::bail!("`dm runs logs --follow` requires a <node_id>.");
    } else {
        let detail = dm_core::runs::get_run(home, &run_id)?;
        println!(
            "Run {} ({})",
            detail.summary.name.bold(),
            detail.summary.id.dimmed()
        );
        if detail.nodes.is_empty() {
            println!("  No log files found.");
        } else {
            println!("  Available node logs:");
            for node in &detail.nodes {
                let size = if node.log_size > 0 {
                    format!("{} bytes", node.log_size)
                } else {
                    "(empty)".to_string()
                };
                println!("    {} {}", node.id.bold(), size.dimmed());
            }
            println!(
                "\n  Use: {} to view a log.",
                format!("dm runs logs {} <node_id>", run_id).cyan()
            );
        }
    }
    Ok(())
}

pub fn clean(home: &Path, keep: usize) -> Result<()> {
    let deleted = dm_core::runs::clean_runs(home, keep)?;
    println!(
        "{} Cleaned {} old run(s), kept most recent {}.",
        "✅".green(),
        deleted,
        keep
    );
    Ok(())
}

async fn follow_run_log(home: &Path, run_id: &str, node_id: &str) -> Result<()> {
    let mut offset = 0u64;

    loop {
        let chunk = dm_core::runs::read_run_log_chunk(home, run_id, node_id, offset)?;
        if !chunk.content.is_empty() {
            print!("{}", chunk.content);
            std::io::stdout().flush()?;
        }

        let no_progress = chunk.next_offset == offset;
        offset = chunk.next_offset;

        if chunk.finished && no_progress {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(())
}
