mod cmd;
mod display;

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{error::ErrorKind, Parser, Subcommand};
use colored::Colorize;
use futures_util::StreamExt;
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

    /// Start a dataflow (alias for start)
    Run {
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

struct CommandSuggestion {
    attempted: &'static str,
    replacement: &'static str,
    reason: &'static str,
}

fn known_command_suggestion(tokens: &[String]) -> Option<CommandSuggestion> {
    match tokens {
        [command, subcommand] if command == "dataflow" && subcommand == "list" => {
            Some(CommandSuggestion {
                attempted: "dataflow list",
                replacement: "dm dataflow",
                reason: "dataflow commands",
            })
        }
        [command] if command == "nodes" => Some(CommandSuggestion {
            attempted: "nodes",
            replacement: "dm node",
            reason: "node commands",
        }),
        [command, subcommand] if command == "node" && subcommand == "list" => {
            Some(CommandSuggestion {
                attempted: "node list",
                replacement: "dm node list",
                reason: "list installed nodes",
            })
        }
        [command, help] if command == "run" && help == "--help" => Some(CommandSuggestion {
            attempted: "run --help",
            replacement: "dm start --help",
            reason: "start a dataflow",
        }),
        _ => None,
    }
}

fn attempted_command_tokens() -> Vec<String> {
    let mut tokens = Vec::new();
    let mut args = std::env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        if tokens.is_empty() {
            match arg.as_str() {
                "--home" => {
                    let _ = args.next();
                    continue;
                }
                "--verbose" | "-v" => continue,
                _ if arg.starts_with("--home=") => continue,
                _ => {}
            }
        }

        tokens.push(arg);
        tokens.extend(args);
        break;
    }

    tokens
}

fn print_command_suggestion(err: &clap::Error) -> bool {
    if err.kind() != ErrorKind::InvalidSubcommand {
        return false;
    }

    let tokens = attempted_command_tokens();
    let Some(suggestion) = known_command_suggestion(&tokens) else {
        return false;
    };

    eprintln!("error: unknown command \"{}\"", suggestion.attempted);
    eprintln!(
        "help: did you mean `{}` ({})?",
        suggestion.replacement, suggestion.reason
    );
    eprintln!("      Run `dm help` to see all available commands.");
    true
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            if print_command_suggestion(&err) {
                std::process::exit(2);
            }
            err.exit();
        }
    };
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

        Commands::Start { file, force } | Commands::Run { file, force } => {
            cmd_start(&home, cli.verbose, &file, force).await?
        }

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

    // Handle URL downloads
    let file_path = if file.starts_with("http://") || file.starts_with("https://") {
        println!(
            "{} Downloading dataflow from {}...",
            "→".cyan(),
            file.dimmed()
        );

        // Create a temporary file for the download
        let mut temp_file = tempfile::Builder::new()
            .suffix(".yml")
            .tempfile()
            .context("Failed to create temporary file")?;

        // Download the file
        let response = reqwest::get(file)
            .await
            .context("Failed to download file from URL")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download file: HTTP {}", response.status());
        }

        // Get content length for progress display
        let total_size = response.content_length();

        // Create progress bar
        let pb = ProgressBar::hidden();
        if let Some(total) = total_size {
            pb.set_length(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  [{bar:30.cyan/dim}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            pb.reset();
        }

        // Stream download to temporary file
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read download chunk")?;
            std::io::Write::write_all(&mut temp_file, &chunk)
                .context("Failed to write to temporary file")?;
            downloaded += chunk.len() as u64;
            if total_size.is_some() {
                pb.set_position(downloaded);
            }
        }

        if total_size.is_some() {
            pb.finish_and_clear();
        }

        println!("{} Download complete!", "✅".green());

        // Persist the temp file so it survives past this block
        let temp_path = temp_file.into_temp_path();
        temp_path
            .keep()
            .context("Failed to persist downloaded file")?
    } else {
        std::path::PathBuf::from(file)
    };

    if !file_path.exists() {
        anyhow::bail!("Graph file '{}' not found.", file_path.display());
    }

    ensure_dm_server_for_dataflow(home, &file_path).await?;

    println!("{} Starting dataflow...", "🚀".green());
    let strategy = if force {
        dm_core::runs::StartConflictStrategy::StopAndRestart
    } else {
        dm_core::runs::StartConflictStrategy::Fail
    };
    let result = dm_core::runs::start_run_from_file_with_source_and_strategy(
        home,
        &file_path,
        None,
        dm_core::runs::RunSource::Cli,
        strategy,
    )
    .await?;
    println!("{} Run created: {}", "✅".green(), result.run.run_id.bold());
    println!(
        "  {} Running in background. Stop with: {}",
        "→".cyan(),
        format!("dm runs stop {}", result.run.run_id).dimmed()
    );
    println!(
        "  {} View in browser: {}",
        "→".cyan(),
        "http://127.0.0.1:3210".dimmed()
    );
    if let Some(dora_uuid) = &result.run.dora_uuid {
        println!("  Dora UUID: {}", dora_uuid.dimmed());
    }
    println!("  {}", result.message);
    Ok(())
}

async fn ensure_dm_server_for_dataflow(
    home: &std::path::Path,
    file_path: &std::path::Path,
) -> Result<()> {
    let yaml = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read graph yaml at {}", file_path.display()))?;

    if !dataflow_requires_dm_server(home, &yaml) || dm_server_ready().await {
        return Ok(());
    }

    println!("dm-server not detected — auto-starting...");
    let _child = std::process::Command::new("dm-server")
        .spawn()
        .context("Failed to auto-start dm-server")?;

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if dm_server_ready().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    anyhow::bail!("dm-server did not become ready within 30 seconds");
}

async fn dm_server_ready() -> bool {
    match reqwest::get("http://127.0.0.1:3210/api/doctor").await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

fn dataflow_requires_dm_server(home: &std::path::Path, yaml: &str) -> bool {
    let detail = dm_core::dataflow::inspect_yaml(home, yaml);
    if detail.summary.requires_media_backend {
        return true;
    }

    let Ok(graph) = serde_yaml::from_str::<serde_yaml::Value>(yaml) else {
        return false;
    };

    if graph.get("services").is_some()
        || graph.get("functions").is_some()
        || graph.get("faas").is_some()
    {
        return true;
    }

    graph
        .get("nodes")
        .and_then(|nodes| nodes.as_sequence())
        .into_iter()
        .flatten()
        .filter_map(|node| node.get("node").and_then(|value| value.as_str()))
        .any(|node_id| node_metadata_requires_dm_server(home, node_id))
}

fn node_metadata_requires_dm_server(home: &std::path::Path, node_id: &str) -> bool {
    let Some(path) = dm_core::node::resolve_dm_json_path(home, node_id) else {
        return false;
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(node) = serde_json::from_str::<dm_core::node::Node>(&content) else {
        return false;
    };

    if node.capabilities.iter().any(|capability| {
        matches!(
            capability.name(),
            "display" | "media" | "widget_input" | "dm_sdk" | "dm-server" | "faas"
        )
    }) {
        return true;
    }

    if node
        .display
        .tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "sdk" | "interaction" | "media" | "faas"))
        || node.description.contains("dm SDK")
        || node.description.contains("dm-server")
    {
        return true;
    }

    serde_json::from_str::<serde_json::Value>(&content)
        .map(|metadata| json_contains_dm_server_env(&metadata))
        .unwrap_or(false)
}

fn json_contains_dm_server_env(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => {
            matches!(value.as_str(), "DM_SERVER_URL" | "DM_FAASD_URL")
        }
        serde_json::Value::Array(values) => values.iter().any(json_contains_dm_server_env),
        serde_json::Value::Object(map) => map.iter().any(|(key, value)| {
            matches!(key.as_str(), "DM_SERVER_URL" | "DM_FAASD_URL")
                || json_contains_dm_server_env(value)
        }),
        _ => false,
    }
}
