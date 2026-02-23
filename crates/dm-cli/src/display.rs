use colored::Colorize;
use dm_core::types::*;

/// Print a section header
pub fn print_header(title: &str) {
    println!();
    println!("{}", title.bold());
    println!("{}", "─".repeat(40).dimmed());
}

/// Print an environment item
pub fn print_env_item(item: &EnvItem) {
    if item.found {
        let ver = item.version.as_deref().unwrap_or("");
        let path = item.path.as_deref().unwrap_or("");
        println!(
            "  {}  {:<14} {} ({})",
            "✅",
            item.name.bold(),
            ver,
            path.dimmed()
        );
    } else {
        let suggestion = item.suggestion.as_deref().unwrap_or("");
        println!(
            "  {}  {:<14} {}",
            "❌",
            item.name.bold(),
            suggestion.yellow()
        );
    }
}

/// Print the full doctor report
pub fn print_doctor_report(report: &DoctorReport) {
    print_header("Dora Manager — Environment Check");

    print_env_item(&report.python);
    print_env_item(&report.uv);
    print_env_item(&report.rust);

    print_header("Dora Installation");

    if report.installed_versions.is_empty() {
        println!(
            "  {}  {:<14} {}",
            "❌",
            "dora".bold(),
            "No versions installed. Run `dm install`.".yellow()
        );
    } else {
        for v in &report.installed_versions {
            let marker = if v.active { " ← active" } else { "" };
            println!("  {}  {}{}", "✅", v.version, marker.green());
        }
    }

    if let Some(ref ver) = report.active_version {
        let status = if report.active_binary_ok {
            "found".dimmed()
        } else {
            "missing!".red()
        };
        println!("\n  {} Active: {} ({})", "→".cyan(), ver.bold(), status);
    }

    println!();
    if report.all_ok {
        println!("  {} Environment is ready.", "✅".green());
    } else {
        println!(
            "  {} Some issues found. Run {} to auto-fix.",
            "⚠️".yellow(),
            "dm setup".bold()
        );
    }
}

/// Print versions report
pub fn print_versions_report(report: &VersionsReport) {
    print_header("Installed");
    if report.installed.is_empty() {
        println!("  (none)");
    } else {
        for v in &report.installed {
            let marker = if v.active { " ← active" } else { "" };
            println!("  • {}{}", v.version.bold(), marker.green());
        }
    }

    print_header("Available (recent)");
    if report.available.is_empty() {
        println!("  {} Could not fetch releases.", "⚠️".yellow());
    } else {
        for r in &report.available {
            let installed_marker = if r.installed { " (installed)" } else { "" };
            println!("  • {}{}", r.tag, installed_marker.dimmed());
        }
    }
}

/// Print status report
pub fn print_status_report(report: &StatusReport) {
    print_header(&format!("Dora Manager v{}", env!("CARGO_PKG_VERSION")));

    match &report.active_version {
        Some(ver) => {
            let actual = report.actual_version.as_deref().unwrap_or("?");
            println!("  dora version:   {} ({})", ver.bold(), actual.dimmed());
        }
        None => {
            println!("  dora version:   {}", "not installed".red());
        }
    }
    println!("  dm home:        {}", report.dm_home.dimmed());

    print_header("Runtime");
    if report.runtime_running {
        for line in report.runtime_output.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                println!("  {}", trimmed);
            }
        }
    } else {
        println!(
            "  {} Coordinator/daemon not running. Use {} to start.",
            "●".red(),
            "dm up".bold()
        );
    }

    print_header("Dataflows");
    if report.dataflows.is_empty() {
        println!("  (no running dataflows)");
    } else {
        for line in &report.dataflows {
            println!("  {}", line);
        }
    }

    println!();
}

/// Print setup report
pub fn print_setup_report(report: &SetupReport) {
    println!();
    println!("  {} Setup complete! Try:", "🎉".green());
    println!("    dm status");
    println!("    dm doctor");
    println!("    dm -- run dataflow.yml --uv");
    if let Some(ref ver) = report.dora_version {
        println!("  Active dora: {}", ver.bold());
    }
    println!();
}

/// Print install result
pub fn print_install_result(result: &InstallResult) {
    if result.set_active {
        println!("  {} Set as active version.", "★".yellow());
    }
    let method = match result.method {
        InstallMethod::Binary => "binary download",
        InstallMethod::Source => "built from source",
    };
    println!(
        "  {} dora {} installed successfully ({}).",
        "✅".green(),
        result.version.bold(),
        method.dimmed()
    );
}

/// Print runtime result (for up/down)
pub fn print_runtime_result(action: &str, result: &RuntimeResult) {
    if result.success {
        println!("  {} {} successful.", "✅".green(), action);
        if !result.message.is_empty() {
            println!("{}", result.message.dimmed());
        }
    } else {
        eprintln!("  {} {} failed.", "❌".red(), action);
        if !result.message.is_empty() {
            eprintln!("{}", result.message);
        }
    }
}
