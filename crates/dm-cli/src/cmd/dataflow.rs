use std::path::Path;

use anyhow::{bail, Result};
use colored::Colorize;

pub async fn import(home: &Path, sources: Vec<String>) -> Result<()> {
    let total = sources.len();
    let mut ok = 0u32;
    let mut failed: Vec<(String, String)> = Vec::new();
    for source in &sources {
        let source_path = Path::new(source);
        let is_url = source.starts_with("https://") || source.starts_with("http://");
        let inferred_name = dm_core::dataflow::infer_import_name(source);

        let result = if is_url {
            println!(
                "{} Importing dataflow {} from git...",
                "→".cyan(),
                inferred_name.bold()
            );
            dm_core::dataflow::import_git(home, &inferred_name, source).await
        } else {
            let abs_path = if source_path.is_absolute() {
                source_path.to_path_buf()
            } else {
                std::env::current_dir()?.join(source_path)
            };
            println!(
                "{} Importing dataflow {} from local...",
                "→".cyan(),
                inferred_name.bold()
            );
            dm_core::dataflow::import_local(home, &inferred_name, &abs_path)
        };

        match result {
            Ok(()) => {
                println!(
                    "{} Imported dataflow {}",
                    "✅".green(),
                    inferred_name.bold()
                );
                ok += 1;
            }
            Err(e) => {
                println!(
                    "{} Failed to import {}: {}",
                    "❌".red(),
                    inferred_name.bold(),
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
        bail!("{} dataflow source(s) failed to import", failed.len());
    }
    Ok(())
}
