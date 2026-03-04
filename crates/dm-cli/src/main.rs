mod display;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use dm_core::types::*;

#[derive(Parser)]
#[command(
    name = "dm",
    version,
    about = "Dora Manager — Bootstrap, manage, and monitor dora-rs environments."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Override dm home directory
    #[arg(long, global = true)]
    home: Option<String>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// One-click bootstrap: install Python, uv, and dora
    Setup,

    /// Check environment health & diagnose issues
    Doctor,

    /// Install a dora version (default: latest)
    Install {
        /// Version to install, e.g. "0.3.9". Omit for latest.
        version: Option<String>,
    },

    /// Remove an installed dora version
    Uninstall {
        /// Version to remove
        version: String,
    },

    /// Switch active dora version
    Use {
        /// Version to activate
        version: String,
    },

    /// Show installed & available dora versions
    Versions,

    /// Start dora coordinator + daemon
    Up,

    /// Stop dora coordinator + daemon
    Down,

    /// Live overview of runtime & dataflows
    Status,

    /// Manage installed dora nodes
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },

    /// Start a dataflow on the running dora runtime
    Start {
        /// Path to dataflow YAML file
        file: String,
    },

    /// View dataflow execution history
    Runs {
        #[command(subcommand)]
        command: Option<RunsCommands>,
    },

    /// Pass-through: run any dora CLI command with the active version
    #[command(
        name = "--",
        trailing_var_arg = true,
        about = "Pass-through to active dora CLI (e.g. dm -- run dataflow.yml --uv)"
    )]
    Passthrough {
        /// Arguments forwarded to dora
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum RunsCommands {
    /// Show logs for a specific run
    Logs {
        /// Dataflow run ID (UUID)
        run_id: String,
        /// Node ID (optional, lists available nodes if omitted)
        node_id: Option<String>,
    },
    /// Clean old run history
    Clean {
        /// Number of recent runs to keep (default: 10)
        #[arg(long, default_value = "10")]
        keep: usize,
    },
}

#[derive(Subcommand)]
enum NodeCommands {
    /// Install node(s) dependencies and build
    Install {
        /// Node id(s) (e.g. dora-yolo dora-keyboard)
        #[arg(required = true)]
        ids: Vec<String>,
    },
    /// Import node(s) from local directories or git URLs
    Import {
        /// Local path(s) or git URL(s)
        #[arg(required = true)]
        sources: Vec<String>,
    },
    /// List installed nodes
    List,
    /// Uninstall node(s)
    Uninstall {
        /// Node id(s)
        #[arg(required = true)]
        ids: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = dm_core::config::resolve_home(cli.home)?;

    match cli.command {
        Commands::Setup => {
            display::print_header("Dora Manager — Setup");
            println!("  Checking prerequisites...\n");

            // Show env checks as we go
            let python = dm_core::env::check_python().await;
            display::print_env_item(&python);
            if !python.found {
                println!("\n  {} Python 3.11+ is required.", "❌".red());
                println!("    macOS:   brew install python@3.11");
                println!("    Linux:   sudo apt install python3.11");
                anyhow::bail!("Python not found. Install it and re-run `dm setup`.");
            }

            let uv = dm_core::env::check_uv().await;
            display::print_env_item(&uv);
            if !uv.found {
                println!("\n  {} Installing uv...", "→".cyan());
            }

            let rust = dm_core::env::check_rust().await;
            display::print_env_item(&rust);

            display::print_header("Dora CLI");
            let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

            let home_clone = home.clone();
            let verbose = cli.verbose;
            let handle = tokio::spawn(async move {
                dm_core::setup(&home_clone, verbose, Some(progress_tx)).await
            });

            // Drain progress messages
            while let Some(progress) = progress_rx.recv().await {
                match &progress.phase {
                    InstallPhase::Fetching => {
                        println!("  {} {}", "→".cyan(), progress.message);
                    }
                    InstallPhase::Downloading {
                        bytes_done: _,
                        bytes_total: _,
                    } => {
                        // Progress bar handled below
                    }
                    InstallPhase::Extracting => {
                        println!("  {} {}", "→".cyan(), progress.message);
                    }
                    InstallPhase::Building => {
                        println!("  {} {}", "→".cyan(), progress.message);
                    }
                    InstallPhase::Done => {
                        println!("  {} {}", "✅".green(), progress.message);
                    }
                }
            }

            let report = handle.await??;
            display::print_setup_report(&report);
        }

        Commands::Doctor => {
            let report = dm_core::doctor(&home).await?;
            display::print_doctor_report(&report);
        }

        Commands::Install { version } => {
            let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

            let home_clone = home.clone();
            let verbose = cli.verbose;
            let handle = tokio::spawn(async move {
                dm_core::install::install(&home_clone, version, verbose, Some(progress_tx)).await
            });

            let pb = ProgressBar::hidden();
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  [{bar:30.cyan/dim}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("█▓░"),
            );

            while let Some(progress) = progress_rx.recv().await {
                match &progress.phase {
                    InstallPhase::Fetching => {
                        println!("{} {}", "→".cyan(), progress.message);
                    }
                    InstallPhase::Downloading {
                        bytes_done,
                        bytes_total,
                    } => {
                        if pb.is_hidden() {
                            pb.set_length(*bytes_total);
                            pb.reset();
                            println!(
                                "{} Downloading ({})...",
                                "→".cyan(),
                                dm_core::util::human_size(*bytes_total)
                            );
                        }
                        pb.set_position(*bytes_done);
                    }
                    InstallPhase::Extracting => {
                        pb.finish_and_clear();
                        println!("{} {}", "→".cyan(), progress.message);
                    }
                    InstallPhase::Building => {
                        println!("{} {}", "→".cyan(), progress.message);
                    }
                    InstallPhase::Done => {}
                }
            }
            pb.finish_and_clear();

            let result = handle.await??;
            display::print_install_result(&result);
        }

        Commands::Uninstall { version } => {
            dm_core::uninstall(&home, &version).await?;
            println!("  {} dora {} removed.", "✅".green(), version.bold());
        }

        Commands::Use { version } => {
            let actual = dm_core::use_version(&home, &version).await?;
            println!(
                "  {} Switched to dora {} ({})",
                "✅".green(),
                version.bold(),
                actual.dimmed()
            );
        }

        Commands::Versions => {
            let report = dm_core::versions(&home).await?;
            display::print_versions_report(&report);
        }

        Commands::Up => {
            println!("{} Starting dora coordinator + daemon...", "→".cyan());
            let result = dm_core::up(&home, cli.verbose).await?;
            display::print_runtime_result("Start", &result);
        }

        Commands::Down => {
            println!("{} Stopping dora coordinator + daemon...", "→".cyan());
            let result = dm_core::down(&home, cli.verbose).await?;
            display::print_runtime_result("Stop", &result);
        }

        Commands::Status => {
            let report = dm_core::status(&home, cli.verbose).await?;
            display::print_status_report(&report);
        }

        Commands::Node { command } => match command {
            NodeCommands::Install { ids } => {
                let total = ids.len();
                let mut ok = 0u32;
                let mut failed: Vec<(String, String)> = Vec::new();
                for id in &ids {
                    println!("{} Installing node {}...", "→".cyan(), id.bold());
                    match dm_core::node::install_node(&home, id).await {
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
            }
            NodeCommands::List => {
                let nodes =
                    dm_core::node::list_nodes(&home).context("Failed to list installed nodes")?;

                if nodes.is_empty() {
                    println!("{} No nodes found.", "ℹ".cyan());
                    println!("  Use {} to import nodes.", "dm node import <path|url>".bold());
                } else {
                    println!("{} Nodes ({})", "📦", nodes.len());
                    println!();
                    for node in &nodes {
                        let name = if node.name.is_empty() { &node.id } else { &node.name };
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
                        let category = if node.category.is_empty() {
                            "".to_string()
                        } else {
                            format!(" [{}]", node.category)
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
            }
            NodeCommands::Import { sources } => {
                let total = sources.len();
                let mut ok = 0u32;
                let mut failed: Vec<(String, String)> = Vec::new();
                for source in &sources {
                    let source_path = std::path::Path::new(source);
                    let is_url = source.starts_with("https://") || source.starts_with("http://");

                    let inferred_id = if is_url {
                        source.rsplit('/').find(|s| !s.is_empty())
                            .unwrap_or("unknown")
                            .to_string()
                    } else {
                        source_path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    };

                    let result = if is_url {
                        println!("{} Importing {} from git...", "→".cyan(), inferred_id.bold());
                        dm_core::node::import_git(&home, &inferred_id, source).await
                    } else {
                        let abs_path = if source_path.is_absolute() {
                            source_path.to_path_buf()
                        } else {
                            std::env::current_dir()?.join(source_path)
                        };
                        println!("{} Importing {} from local...", "→".cyan(), inferred_id.bold());
                        dm_core::node::import_local(&home, &inferred_id, &abs_path)
                    };

                    match result {
                        Ok(node) => {
                            println!("{} Imported {} ({})", "✅".green(), node.name.bold(), node.id.dimmed());
                            println!("  Build: {}", node.source.build.dimmed());
                            ok += 1;
                        }
                        Err(e) => {
                            println!("{} Failed to import {}: {}", "❌".red(), inferred_id.bold(), e);
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
            }
            NodeCommands::Uninstall { ids } => {
                let total = ids.len();
                let mut ok = 0u32;
                let mut failed: Vec<(String, String)> = Vec::new();
                for id in &ids {
                    match dm_core::node::uninstall_node(&home, id) {
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
            }
        },

        Commands::Start { file } => {
            // Check if runtime is running first
            if !dm_core::is_runtime_running(&home, cli.verbose).await {
                eprintln!("{} Dora runtime is not running.", "✗".red());
                eprintln!(
                    "  Run {} first to start the coordinator and daemon.",
                    "dm up".bold()
                );
                std::process::exit(1);
            }

            let file_path = std::path::Path::new(&file);
            if !file_path.exists() {
                anyhow::bail!("Graph file '{}' not found.", file);
            }

            let dataflow_name = file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            println!("{} Translating graph with dm transpiler...", "→".cyan());
            let transpiled = dm_core::dataflow::transpile_graph(&home, file_path)
                .with_context(|| format!("Failed to transpile '{}'", file))?;

            // Write to a temporary run file
            let run_dir = home.join("run");
            std::fs::create_dir_all(&run_dir)?;
            let temp_run_file = run_dir.join(format!(
                ".run_{}",
                file_path.file_name().unwrap_or_default().to_string_lossy()
            ));

            let out_content = serde_yaml::to_string(&transpiled)?;
            std::fs::write(&temp_run_file, out_content)?;

            if cli.verbose {
                println!(
                    "{} Transpiled graph saved to: {}",
                    "ℹ".cyan(),
                    temp_run_file.display()
                );
            }

            println!("{} Starting dataflow...", "🚀".green());
            let args = vec![
                "start".to_string(),
                temp_run_file.to_string_lossy().to_string(),
            ];

            // Ignore SIGINT in this process so only the dora child responds to Ctrl+C.
            // After dora exits, we can still run collect_run_events.
            let _ = ctrlc::set_handler(|| {
                // Do nothing — let the dora child process handle the signal.
            });

            let (code, dataflow_id) =
                dm_core::dora::exec_dora_capture_id(&home, &args, cli.verbose).await?;

            // Collect run events from dora's out/ directory
            if let Some(ref df_id) = dataflow_id {
                dm_core::dataflow::collect_run_events(&home, df_id, &dataflow_name, code);
                if cli.verbose {
                    println!(
                        "{} Events recorded for dataflow {} ({})",
                        "ℹ".cyan(),
                        dataflow_name.bold(),
                        df_id.dimmed()
                    );
                }
            }

            std::process::exit(code);
        }

        Commands::Runs { command } => {
            match command {
                None => {
                    // Default: list recent runs
                    let result = dm_core::runs::list_runs(&home, 20, 0)?;
                    if result.runs.is_empty() {
                        println!("No dataflow runs recorded yet.");
                    } else {
                        println!(
                            "{:<40} {:<15} {:<22} {:<6} {}",
                            "ID", "Name", "Started", "Exit", "Nodes"
                        );
                        println!("{}", "─".repeat(90));
                        for run in &result.runs {
                            let status_icon = match run.exit_code {
                                Some(0) => "✅".to_string(),
                                Some(c) => format!("❌ {}", c),
                                None => "⏳".to_string(),
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
                        println!(
                            "\nShowing {}/{} runs.",
                            result.runs.len(),
                            result.total
                        );
                    }
                }
                Some(RunsCommands::Logs { run_id, node_id }) => {
                    if let Some(nid) = node_id {
                        let content = dm_core::runs::get_run_logs(&home, &run_id, &nid)?;
                        if content.is_empty() {
                            println!("(empty log)");
                        } else {
                            print!("{}", content);
                        }
                    } else {
                        // No node_id: show run detail with available nodes
                        let detail = dm_core::runs::get_run(&home, &run_id)?;
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
                }
                Some(RunsCommands::Clean { keep }) => {
                    let deleted = dm_core::runs::clean_runs(&home, keep)?;
                    println!(
                        "{} Cleaned {} old run(s), kept most recent {}.",
                        "✅".green(),
                        deleted,
                        keep
                    );
                }
            }
        }

        Commands::Passthrough { args } => {
            let code = dm_core::passthrough(&home, &args, cli.verbose).await?;
            std::process::exit(code);
        }
    }

    Ok(())
}
