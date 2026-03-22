mod builtin;
mod cmd;
mod display;

use anyhow::Result;
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

    /// Test a single node interactively
    #[command(subcommand)]
    Test(TestCommands),

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
enum TestCommands {
    /// [internal] Run as dora harness node (called by transpiler)
    HarnessServe {
        /// Auto-trigger all SUT input ports on startup
        #[arg(long)]
        auto_trigger: bool,
        /// Output port names (harness → SUT inputs)
        #[arg(long, value_delimiter = ',')]
        output_ports: Vec<String>,
    },
    /// Test a node by name (generates test dataflow + runs it)
    Run {
        /// Node ID (e.g. dm-downloader)
        node_id: String,
        /// Override config values (repeatable)
        #[arg(long = "config", value_name = "KEY=VALUE")]
        config_overrides: Vec<String>,
        /// Auto-trigger all input ports on startup
        #[arg(long)]
        auto_trigger: bool,
        /// Auto-exit after N seconds (0 = manual/interactive)
        #[arg(long, default_value = "0")]
        timeout: u64,
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

// ---------------------------------------------------------------------------
// Main dispatch
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = dm_core::config::resolve_home(cli.home)?;

    match cli.command {
        Commands::Setup => cmd_setup(&home, cli.verbose).await?,
        Commands::Doctor => {
            let report = dm_core::doctor(&home).await?;
            display::print_doctor_report(&report);
        }
        Commands::Install { version } => cmd_install(&home, cli.verbose, version).await?,
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

        // --- Delegated command groups ---
        Commands::Node { command } => match command {
            NodeCommands::Install { ids } => cmd::node::install(&home, ids).await?,
            NodeCommands::List => cmd::node::list(&home)?,
            NodeCommands::Import { sources } => cmd::node::import(&home, sources).await?,
            NodeCommands::Uninstall { ids } => cmd::node::uninstall(&home, ids)?,
        },

        Commands::Dataflow { command } => match command {
            DataflowCommands::Import { sources } => cmd::dataflow::import(&home, sources).await?,
        },

        Commands::Start { file, force } => cmd_start(&home, cli.verbose, &file, force).await?,

        Commands::Runs { command } => match command {
            None => cmd::runs::list(&home).await?,
            Some(RunsCommands::Stop { run_id }) => cmd::runs::stop(&home, run_id).await?,
            Some(RunsCommands::Delete { run_ids }) => cmd::runs::delete(&home, run_ids)?,
            Some(RunsCommands::Logs {
                run_id,
                node_id,
                follow,
            }) => cmd::runs::logs(&home, run_id, node_id, follow).await?,
            Some(RunsCommands::Clean { keep }) => cmd::runs::clean(&home, keep)?,
        },

        Commands::Panel(command) => match command {
            PanelCommands::Serve { run_id, node_id } => {
                builtin::panel::panel_serve(&home, &run_id, &node_id)?;
            }
            PanelCommands::Send {
                output_id,
                value,
                run,
            } => {
                builtin::panel::panel_send(&home, &output_id, &value, run)?;
            }
        },

        Commands::Test(command) => match command {
            TestCommands::HarnessServe {
                auto_trigger,
                output_ports,
            } => {
                cmd::test::harness_serve(auto_trigger, &output_ports)?;
            }
            TestCommands::Run {
                node_id,
                config_overrides,
                auto_trigger,
                timeout,
            } => {
                cmd::test::run(
                    &home,
                    cli.verbose,
                    &node_id,
                    &config_overrides,
                    auto_trigger,
                    timeout,
                )
                .await?;
            }
        },

        Commands::Passthrough { args } => {
            let code = dm_core::passthrough(&home, &args, cli.verbose).await?;
            std::process::exit(code);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Inline handlers (too small to extract to a file)
// ---------------------------------------------------------------------------

async fn cmd_setup(home: &std::path::Path, verbose: bool) -> Result<()> {
    display::print_header("Dora Manager — Setup");
    println!("  Checking prerequisites...\n");

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

    let home_clone = home.to_path_buf();
    let handle =
        tokio::spawn(async move { dm_core::setup(&home_clone, verbose, Some(progress_tx)).await });

    while let Some(progress) = progress_rx.recv().await {
        match &progress.phase {
            InstallPhase::Fetching => println!("  {} {}", "→".cyan(), progress.message),
            InstallPhase::Downloading { .. } => {}
            InstallPhase::Extracting => println!("  {} {}", "→".cyan(), progress.message),
            InstallPhase::Building => println!("  {} {}", "→".cyan(), progress.message),
            InstallPhase::Done => println!("  {} {}", "✅".green(), progress.message),
        }
    }

    let report = handle.await??;
    display::print_setup_report(&report);
    Ok(())
}

async fn cmd_install(home: &std::path::Path, verbose: bool, version: Option<String>) -> Result<()> {
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

    let home_clone = home.to_path_buf();
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
            InstallPhase::Fetching => println!("{} {}", "→".cyan(), progress.message),
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
            InstallPhase::Building => println!("{} {}", "→".cyan(), progress.message),
            InstallPhase::Done => {}
        }
    }
    pb.finish_and_clear();

    let result = handle.await??;
    display::print_install_result(&result);
    Ok(())
}

async fn cmd_start(home: &std::path::Path, verbose: bool, file: &str, force: bool) -> Result<()> {
    if !dm_core::is_runtime_running(home, verbose).await {
        println!("{} Dora runtime not running, starting...", "→".cyan());
    }
    dm_core::ensure_runtime_up(home, verbose).await?;

    let file_path = std::path::Path::new(file);
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
        home,
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
    Ok(())
}
