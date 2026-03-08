mod display;

use std::io::Write;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use dora_node_api::arrow::array::{
    BooleanArray, Float64Array, Int64Array, StringArray, UInt8Array,
};
use dora_node_api::arrow::datatypes::DataType;

use dora_node_api::{DoraNode, Event};
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

    /// Manage dataflow projects
    Dataflow {
        #[command(subcommand)]
        command: DataflowCommands,
    },

    /// Start a dataflow on the running dora runtime
    Start {
        /// Path to dataflow YAML file
        file: String,
        /// Stop an active run with the same dataflow name before starting
        #[arg(long)]
        force: bool,
    },

    /// View dataflow execution history
    Runs {
        #[command(subcommand)]
        command: Option<RunsCommands>,
    },

    /// Panel: real-time visualization, control, and recording
    #[command(subcommand)]
    Panel(PanelCommands),

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
    /// Stop a specific run by DM run ID
    Stop {
        /// Run ID
        run_id: String,
    },
    /// Delete one or more runs by DM run ID
    Delete {
        /// One or more run IDs
        #[arg(required = true)]
        run_ids: Vec<String>,
    },
    /// Show logs for a specific run
    Logs {
        /// Dataflow run ID (UUID)
        run_id: String,
        /// Node ID (optional, lists available nodes if omitted)
        node_id: Option<String>,
        /// Continuously print appended log output until the run finishes
        #[arg(long)]
        follow: bool,
    },
    /// Clean old run history
    Clean {
        /// Number of recent runs to keep (default: 10)
        #[arg(long, default_value = "10")]
        keep: usize,
    },
}

#[derive(Subcommand)]
enum DataflowCommands {
    /// Import dataflow project(s) from local paths or GitHub URLs
    Import {
        /// Local path(s) or GitHub URL(s)
        #[arg(required = true)]
        sources: Vec<String>,
    },
}

#[derive(Subcommand)]
enum PanelCommands {
    /// [internal] Run as dora node (called by transpiler)
    Serve {
        /// Run ID (UUID)
        #[arg(long)]
        run_id: String,
        /// Node ID in the dataflow graph
        #[arg(long, default_value = "dm-panel")]
        node_id: String,
    },
    /// Send a control command to the active panel
    Send {
        /// Output ID (e.g. "speed", "direction")
        output_id: String,
        /// Value (JSON format)
        value: String,
        /// Specific run ID (auto-discovered if omitted)
        #[arg(long)]
        run: Option<String>,
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
                    println!(
                        "  Use {} to import nodes.",
                        "dm node import <path|url>".bold()
                    );
                } else {
                    println!("{} Nodes ({})", "📦", nodes.len());
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
            }
            NodeCommands::Import { sources } => {
                let total = sources.len();
                let mut ok = 0u32;
                let mut failed: Vec<(String, String)> = Vec::new();
                for source in &sources {
                    let source_path = std::path::Path::new(source);
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
                        dm_core::node::import_git(&home, &inferred_id, source).await
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
                        dm_core::node::import_local(&home, &inferred_id, &abs_path)
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

        Commands::Dataflow { command } => match command {
            DataflowCommands::Import { sources } => {
                let total = sources.len();
                let mut ok = 0u32;
                let mut failed: Vec<(String, String)> = Vec::new();
                for source in &sources {
                    let source_path = std::path::Path::new(source);
                    let is_url = source.starts_with("https://") || source.starts_with("http://");
                    let inferred_name = dm_core::dataflow::infer_import_name(source);

                    let result = if is_url {
                        println!(
                            "{} Importing dataflow {} from git...",
                            "→".cyan(),
                            inferred_name.bold()
                        );
                        dm_core::dataflow::import_git(&home, &inferred_name, source).await
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
                        dm_core::dataflow::import_local(&home, &inferred_name, &abs_path)
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
            }
        },

        Commands::Start { file, force } => {
            // Auto-start runtime if not running
            if !dm_core::is_runtime_running(&home, cli.verbose).await {
                println!("{} Dora runtime not running, starting...", "→".cyan());
            }
            dm_core::ensure_runtime_up(&home, cli.verbose).await?;

            let file_path = std::path::Path::new(&file);
            if !file_path.exists() {
                anyhow::bail!("Graph file '{}' not found.", file);
            }

            println!("{} Starting dataflow...", "🚀".green());
            let strategy = if force {
                dm_core::runs::StartConflictStrategy::StopAndRestart
            } else {
                dm_core::runs::StartConflictStrategy::Fail
            };
            let result = dm_core::runs::start_run_from_file_with_source_and_strategy(
                &home,
                file_path,
                dm_core::runs::RunSource::Cli,
                strategy,
            )
            .await?;
            println!("{} Run created: {}", "✅".green(), result.run.run_id.bold());
            if let Some(dora_uuid) = &result.run.dora_uuid {
                println!("  Dora UUID: {}", dora_uuid.dimmed());
            }
            println!("  {}", result.message);
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
                }
                Some(RunsCommands::Stop { run_id }) => {
                    let run = dm_core::runs::stop_run(&home, &run_id).await?;
                    println!("{} Stopped run {}", "✅".green(), run.run_id.bold());
                    if let Some(stopped_at) = run.stopped_at {
                        println!("  Stopped at: {}", stopped_at.dimmed());
                    }
                }
                Some(RunsCommands::Delete { run_ids }) => {
                    let total = run_ids.len();
                    let mut deleted = 0usize;
                    let mut failed = Vec::new();

                    for run_id in run_ids {
                        match dm_core::runs::delete_run(&home, &run_id) {
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
                }
                Some(RunsCommands::Logs {
                    run_id,
                    node_id,
                    follow,
                }) => {
                    if let Some(nid) = node_id {
                        if follow {
                            follow_run_log(&home, &run_id, &nid).await?;
                        } else {
                            let content = dm_core::runs::read_run_log(&home, &run_id, &nid)?;
                            if content.is_empty() {
                                println!("(empty log)");
                            } else {
                                print!("{}", content);
                            }
                        }
                    } else if follow {
                        anyhow::bail!("`dm runs logs --follow` requires a <node_id>.");
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

        Commands::Panel(command) => match command {
            PanelCommands::Serve { run_id, node_id } => {
                panel_serve(&home, &run_id, &node_id)?;
            }
            PanelCommands::Send {
                output_id,
                value,
                run,
            } => {
                panel_send(&home, &output_id, &value, run)?;
            }
        },

        Commands::Passthrough { args } => {
            let code = dm_core::passthrough(&home, &args, cli.verbose).await?;
            std::process::exit(code);
        }
    }

    Ok(())
}

fn panel_serve(home: &std::path::Path, run_id: &str, _node_id: &str) -> Result<()> {
    let store = dm_core::runs::panel::PanelStore::open(home, run_id)?;
    let (mut node, mut events) =
        DoraNode::init_from_env().map_err(|e| anyhow::anyhow!("Failed to init panel node: {e}"))?;
    let mut last_cmd_seq = 0i64;
    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let store2 = store.clone();
    let stop_flag = should_stop.clone();
    let reader = std::thread::spawn(move || {
        let mut saw_stop = false;
        while let Some(event) = events.recv() {
            match event {
                Event::Input { id, metadata, data } => {
                    let type_hint = extract_type_hint(&metadata, &data);
                    let bytes = arrow_to_bytes(&metadata, &data);
                    if let Err(e) = store2.write_asset(&id.to_string(), &type_hint, &bytes) {
                        eprintln!("Panel write error: {e}");
                    }
                }
                Event::Stop(_) => {
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    saw_stop = true;
                }
                _ => {}
            }
            if saw_stop {
                continue;
            }
        }
        stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    loop {
        if should_stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        for cmd in store.poll_commands(&mut last_cmd_seq)? {
            send_json_command(&mut node, &cmd.output_id, &cmd.value)
                .with_context(|| format!("Failed sending output '{}'", cmd.output_id))?;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    reader
        .join()
        .map_err(|_| anyhow::anyhow!("Panel event reader thread panicked"))?;

    Ok(())
}

async fn follow_run_log(home: &std::path::Path, run_id: &str, node_id: &str) -> Result<()> {
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

fn panel_send(
    home: &std::path::Path,
    output_id: &str,
    value: &str,
    run: Option<String>,
) -> Result<()> {
    let run_id = match run {
        Some(id) => {
            let db_path = dm_core::runs::run_panel_dir(home, &id).join("index.db");
            if !db_path.exists() {
                bail!("Panel run '{}' not found", id);
            }
            id
        }
        None => {
            let runs = dm_core::runs::refresh_run_statuses(home)?;
            let mut active = runs
                .into_iter()
                .filter(|run| run.status.is_running() && run.has_panel)
                .collect::<Vec<_>>();
            active.sort_by(|a, b| b.started_at.cmp(&a.started_at));

            match active.len() {
                0 => bail!("No active run with panel found"),
                1 => active[0].run_id.clone(),
                _ => {
                    eprintln!("Multiple active runs with panel found; using most recent:");
                    for run in &active {
                        eprintln!("  {} ({})", run.run_id, run.started_at);
                    }
                    active[0].run_id.clone()
                }
            }
        }
    };

    let store = dm_core::runs::panel::PanelStore::open(home, &run_id)?;
    store.write_command(output_id, value)?;
    println!("✅ Sent: {} = {}", output_id, value);
    Ok(())
}

fn extract_type_hint(
    metadata: &dora_node_api::Metadata,
    data: &dora_node_api::ArrowData,
) -> String {
    if let Some(dora_node_api::Parameter::String(content_type)) =
        metadata.parameters.get("content_type")
    {
        return content_type.clone();
    }
    if sample_rate_from_metadata(metadata).is_some() && is_audio_array(data.data_type()) {
        return "audio/wav".to_string();
    }
    match data.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => "text/plain".to_string(),
        DataType::Binary | DataType::LargeBinary => "application/octet-stream".to_string(),
        DataType::UInt8 => "application/octet-stream".to_string(),
        _ => format!("application/x-arrow+{:?}", data.data_type()).to_ascii_lowercase(),
    }
}

fn arrow_to_bytes(metadata: &dora_node_api::Metadata, data: &dora_node_api::ArrowData) -> Vec<u8> {
    if let Some(sample_rate) = sample_rate_from_metadata(metadata) {
        if let Some(bytes) = audio_array_to_wav_bytes(sample_rate, data) {
            return bytes;
        }
    }
    match data.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => String::try_from(data)
            .map(|s| s.into_bytes())
            .unwrap_or_else(|_| format!("{data:?}").into_bytes()),
        DataType::UInt8 => Vec::<u8>::try_from(data).unwrap_or_default(),
        DataType::Binary | DataType::LargeBinary => format!("{data:?}").into_bytes(),
        _ => format!("{data:?}").into_bytes(),
    }
}

fn sample_rate_from_metadata(metadata: &dora_node_api::Metadata) -> Option<u32> {
    match metadata.parameters.get("sample_rate") {
        Some(dora_node_api::Parameter::Integer(v)) if *v > 0 => Some(*v as u32),
        Some(dora_node_api::Parameter::Float(v)) if *v > 0.0 => Some(*v as u32),
        _ => None,
    }
}

fn is_audio_array(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Float32
            | DataType::Float64
            | DataType::Int16
            | DataType::Int32
            | DataType::UInt16
            | DataType::UInt32
    )
}

fn audio_array_to_wav_bytes(sample_rate: u32, data: &dora_node_api::ArrowData) -> Option<Vec<u8>> {
    let samples = match data.data_type() {
        DataType::Float32 => {
            let values = Vec::<f32>::try_from(data).ok()?;
            values
                .into_iter()
                .map(|v| (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .collect::<Vec<_>>()
        }
        DataType::Float64 => {
            let values = Vec::<f64>::try_from(data).ok()?;
            values
                .into_iter()
                .map(|v| (v.clamp(-1.0, 1.0) * i16::MAX as f64) as i16)
                .collect::<Vec<_>>()
        }
        _ => return None,
    };

    Some(encode_wav_mono_i16(sample_rate, &samples))
}

fn encode_wav_mono_i16(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let bits_per_sample = 16u16;
    let channels = 1u16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_len = (samples.len() * std::mem::size_of::<i16>()) as u32;
    let riff_len = 36 + data_len;

    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_len.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

fn send_json_command(node: &mut DoraNode, output_id: &str, raw_json: &str) -> Result<()> {
    let value = serde_json::from_str::<serde_json::Value>(raw_json)
        .unwrap_or_else(|_| serde_json::Value::String(raw_json.to_string()));

    match value {
        serde_json::Value::Bool(v) => {
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                BooleanArray::from(vec![v]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        serde_json::Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Int64Array::from(vec![i]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else if let Some(f) = num.as_f64() {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Float64Array::from(vec![f]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    StringArray::from(vec![num.to_string()]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            }
        }
        serde_json::Value::String(s) => {
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                StringArray::from(vec![s]),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        serde_json::Value::Array(items) => {
            if items.iter().all(|v| v.as_i64().is_some()) {
                let values = items
                    .into_iter()
                    .map(|v| v.as_i64().unwrap_or_default())
                    .collect::<Vec<_>>();
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Int64Array::from(values),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else if items.iter().all(|v| v.as_f64().is_some()) {
                let values = items
                    .into_iter()
                    .map(|v| v.as_f64().unwrap_or_default())
                    .collect::<Vec<_>>();
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    Float64Array::from(values),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else if items.iter().all(|v| v.as_str().is_some()) {
                let values = items
                    .into_iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect::<Vec<_>>();
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    StringArray::from(values),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            } else {
                node.send_output(
                    output_id.to_string().into(),
                    Default::default(),
                    StringArray::from(vec![serde_json::Value::Array(items).to_string()]),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            }
        }
        serde_json::Value::Null | serde_json::Value::Object(_) => {
            let bytes = value.to_string().into_bytes();
            node.send_output(
                output_id.to_string().into(),
                Default::default(),
                UInt8Array::from(bytes),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
    }
    Ok(())
}
