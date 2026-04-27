use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

pub fn create(home: &Path, id: &str, description: &str) -> Result<()> {
    let service = dm_core::service::create_service(home, id, description)?;
    println!("{} Created service {}", "✅".green(), service.id.bold());
    println!("  Path: {}", service.path.display().to_string().dimmed());
    Ok(())
}

pub async fn install(home: &Path, ids: Vec<String>) -> Result<()> {
    let total = ids.len();
    let mut ok = 0u32;
    let mut failed: Vec<(String, String)> = Vec::new();

    for id in &ids {
        println!("{} Installing service {}...", "→".cyan(), id.bold());
        match dm_core::service::install_service(home, id).await {
            Ok(service) => {
                println!(
                    "{} Installed service {} ({})",
                    "✅".green(),
                    service.display_name().bold(),
                    service.id.dimmed()
                );
                if let Some(exec) = service.runtime.exec.as_deref() {
                    println!("  Exec: {}", exec.dimmed());
                }
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
        println!("Done: {}/{} installed.", ok, total);
    }
    if !failed.is_empty() {
        bail!("{} service(s) failed to install", failed.len());
    }
    Ok(())
}

pub fn list(home: &Path) -> Result<()> {
    let services =
        dm_core::service::list_services(home).context("Failed to list available services")?;

    if services.is_empty() {
        println!("{} No services found.", "ℹ".cyan());
        return Ok(());
    }

    println!("🧩 Services ({})", services.len());
    println!();
    for service in &services {
        let scope = format!("{:?}", service.scope).to_lowercase();
        let runtime = format!("{:?}", service.runtime.kind).to_lowercase();
        let builtin = if service.builtin { " builtin" } else { "" };
        let category = if service.display.category.is_empty() {
            String::new()
        } else {
            format!(" {}", format!("[{}]", service.display.category).dimmed())
        };
        println!(
            "  {} {} v{}{} [{} / {}{}]",
            "•".cyan(),
            service.display_name().bold(),
            service.version.dimmed(),
            category,
            scope.dimmed(),
            runtime.dimmed(),
            builtin.dimmed()
        );
        if service.display_name() != service.id {
            println!("    id: {}", service.id.dimmed());
        }
        if !service.description.is_empty() {
            println!("    {}", service.description.dimmed());
        }
        if !service.methods.is_empty() {
            let methods = service
                .methods
                .iter()
                .map(|method| method.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            println!("    methods: {}", methods);
        }
    }

    Ok(())
}

pub fn describe(home: &Path, id: &str) -> Result<()> {
    let Some(service) =
        dm_core::service::get_service(home, id).context("Failed to describe service")?
    else {
        bail!("Service '{}' was not found", id);
    };

    let scope = format!("{:?}", service.scope).to_lowercase();
    let runtime = format!("{:?}", service.runtime.kind).to_lowercase();

    println!(
        "🧩 {} ({})",
        service.display_name().bold(),
        service.id.dimmed()
    );
    println!("  Version: {}", service.version);
    println!("  Scope: {}", scope);
    println!("  Runtime: {}", runtime);
    if !service.display.category.is_empty() {
        println!("  Category: {}", service.display.category);
    }
    if !service.display.tags.is_empty() {
        println!("  Tags: {}", service.display.tags.join(", "));
    }
    if !service.files.readme.is_empty() {
        println!("  README: {}", service.files.readme);
    }
    if service.builtin {
        println!("  Builtin: yes");
    }
    if !service.description.is_empty() {
        println!();
        println!("{}", service.description);
    }

    if service.methods.is_empty() {
        println!();
        println!("No methods declared.");
        return Ok(());
    }

    println!();
    println!("Methods:");
    for method in &service.methods {
        println!("  - {}", method.name.bold());
        if !method.description.is_empty() {
            println!("    {}", method.description.dimmed());
        }
        if method.input_schema.is_some() {
            println!("    input_schema: declared");
        }
        if method.output_schema.is_some() {
            println!("    output_schema: declared");
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
        let inferred_id = infer_service_import_id(source_path, source, is_url);

        let result = if is_url {
            println!(
                "{} Importing service {} from git...",
                "→".cyan(),
                inferred_id.bold()
            );
            dm_core::service::import_git(home, &inferred_id, source).await
        } else {
            let abs_path = if source_path.is_absolute() {
                source_path.to_path_buf()
            } else {
                std::env::current_dir()?.join(source_path)
            };
            println!(
                "{} Importing service {} from local...",
                "→".cyan(),
                inferred_id.bold()
            );
            dm_core::service::import_local(home, &inferred_id, &abs_path)
        };

        match result {
            Ok(service) => {
                println!(
                    "{} Imported service {} ({})",
                    "✅".green(),
                    service.display_name().bold(),
                    service.id.dimmed()
                );
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
        bail!("{} service source(s) failed to import", failed.len());
    }
    Ok(())
}

pub fn uninstall(home: &Path, ids: Vec<String>) -> Result<()> {
    let total = ids.len();
    let mut ok = 0u32;
    let mut failed: Vec<(String, String)> = Vec::new();

    for id in &ids {
        match dm_core::service::uninstall_service(home, id) {
            Ok(()) => {
                println!("{} Service {} removed.", "✅".green(), id.bold());
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
        bail!("{} service(s) failed to uninstall", failed.len());
    }
    Ok(())
}

pub fn readme(home: &Path, id: &str) -> Result<()> {
    let content = dm_core::service::get_service_readme(home, id)
        .with_context(|| format!("Failed to read README for service '{}'", id))?;
    println!("{}", content);
    Ok(())
}

pub fn files(home: &Path, id: &str) -> Result<()> {
    let files = dm_core::service::git_like_file_tree(home, id)
        .with_context(|| format!("Failed to list files for service '{}'", id))?;
    for file in files {
        println!("{}", file);
    }
    Ok(())
}

pub fn file(home: &Path, id: &str, file_path: &str) -> Result<()> {
    let content = dm_core::service::read_service_file(home, id, file_path)
        .with_context(|| format!("Failed to read service file '{}'", file_path))?;
    println!("{}", content);
    Ok(())
}

pub fn config(home: &Path, id: &str) -> Result<()> {
    let config = dm_core::service::get_service_config(home, id)
        .with_context(|| format!("Failed to read config for service '{}'", id))?;
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}

pub fn set_config(home: &Path, id: &str, raw_json: &str) -> Result<()> {
    let config = serde_json::from_str::<serde_json::Value>(raw_json)
        .with_context(|| "Config must be valid JSON")?;
    dm_core::service::save_service_config(home, id, &config)
        .with_context(|| format!("Failed to save config for service '{}'", id))?;
    println!("{} Config saved for service {}.", "✅".green(), id.bold());
    Ok(())
}

fn infer_service_import_id(source_path: &Path, source: &str, is_url: bool) -> String {
    if is_url {
        return source
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or("unknown")
            .to_string();
    }

    let manifest_path = source_path.join("service.json");
    if let Ok(content) = std::fs::read_to_string(manifest_path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
                return id.to_string();
            }
        }
    }

    source_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}
