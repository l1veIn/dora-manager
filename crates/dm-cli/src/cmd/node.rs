use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

pub async fn install(home: &Path, ids: Vec<String>) -> Result<()> {
    let total = ids.len();
    let mut ok = 0u32;
    let mut failed: Vec<(String, String)> = Vec::new();
    for id in &ids {
        println!("{} Installing node {}...", "→".cyan(), id.bold());
        match dm_core::node::install_node(home, id).await {
            Ok(entry) => {
                println!(
                    "{} Installed {} ({})",
                    "✅".green(),
                    entry.id.bold(),
                    entry.version.green()
                );
                println!("  Path: {}", entry.path.display().to_string().dimmed());
                ok += 1;
            }
            Err(e) => {
                println!("{} Failed to install {}: {}", "❌".red(), id.bold(), e);
                failed.push((id.clone(), format!("{}", e)));
            }
        }
    }
    if total > 1 {
        println!();
        println!("Done: {}/{} succeeded.", ok, total);
    }
    if !failed.is_empty() {
        bail!("{} node(s) failed to install", failed.len());
    }
    Ok(())
}

pub fn list(home: &Path) -> Result<()> {
    let nodes = dm_core::node::list_nodes(home).context("Failed to list installed nodes")?;

    if nodes.is_empty() {
        println!("{} No nodes found.", "ℹ".cyan());
        println!(
            "  Use {} to import nodes.",
            "dm node import <path|url>".bold()
        );
    } else {
        println!("📦 Nodes ({})", nodes.len());
        println!();
        for node in &nodes {
            let name = if node.name.is_empty() {
                &node.id
            } else {
                &node.name
            };
            let installed = !node.executable.is_empty();
            let status = if installed {
                "✅".to_string()
            } else {
                "⬇".to_string()
            };
            let version = if node.version.is_empty() {
                "".to_string()
            } else {
                format!(" v{}", node.version)
            };
            let category = if node.display.category.is_empty() {
                "".to_string()
            } else {
                format!(" [{}]", node.display.category)
            };
            println!(
                "  {} {}{}{} {}",
                status,
                name.bold(),
                version.dimmed(),
                category.dimmed(),
                if installed { "" } else { "(not installed)" }.yellow()
            );
            if !node.description.is_empty() {
                println!("    {}", node.description.dimmed());
            }
        }
    }
    Ok(())
}

pub async fn import(home: &Path, sources: Vec<String>) -> Result<()> {
    let total = sources.len();
    let mut ok = 0u32;
    let mut failed: Vec<(String, String)> = Vec::new();
    for source in &sources {
        let source_path = Path::new(source);
        let is_url = source.starts_with("https://") || source.starts_with("http://");

        let inferred_id = if is_url {
            source
                .rsplit('/')
                .find(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string()
        } else {
            source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };

        let result = if is_url {
            println!(
                "{} Importing {} from git...",
                "→".cyan(),
                inferred_id.bold()
            );
            dm_core::node::import_git(home, &inferred_id, source).await
        } else {
            let abs_path = if source_path.is_absolute() {
                source_path.to_path_buf()
            } else {
                std::env::current_dir()?.join(source_path)
            };
            println!(
                "{} Importing {} from local...",
                "→".cyan(),
                inferred_id.bold()
            );
            dm_core::node::import_local(home, &inferred_id, &abs_path)
        };

        match result {
            Ok(node) => {
                println!(
                    "{} Imported {} ({})",
                    "✅".green(),
                    node.name.bold(),
                    node.id.dimmed()
                );
                println!("  Build: {}", node.source.build.dimmed());
                ok += 1;
            }
            Err(e) => {
                println!(
                    "{} Failed to import {}: {}",
                    "❌".red(),
                    inferred_id.bold(),
                    e
                );
                failed.push((source.clone(), format!("{}", e)));
            }
        }
    }
    if total > 1 {
        println!();
        println!("Done: {}/{} imported.", ok, total);
    }
    if !failed.is_empty() {
        bail!("{} source(s) failed to import", failed.len());
    }
    Ok(())
}

pub fn uninstall(home: &Path, ids: Vec<String>) -> Result<()> {
    let total = ids.len();
    let mut ok = 0u32;
    let mut failed: Vec<(String, String)> = Vec::new();
    for id in &ids {
        match dm_core::node::uninstall_node(home, id) {
            Ok(()) => {
                println!("{} Node {} removed.", "✅".green(), id.bold());
                ok += 1;
            }
            Err(e) => {
                println!("{} Failed to uninstall {}: {}", "❌".red(), id.bold(), e);
                failed.push((id.clone(), format!("{}", e)));
            }
        }
    }
    if total > 1 {
        println!();
        println!("Done: {}/{} removed.", ok, total);
    }
    if !failed.is_empty() {
        bail!("{} node(s) failed to uninstall", failed.len());
    }
    Ok(())
}
