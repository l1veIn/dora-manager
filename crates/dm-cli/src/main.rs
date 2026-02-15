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

        Commands::Passthrough { args } => {
            let code = dm_core::passthrough(&home, &args, cli.verbose).await?;
            std::process::exit(code);
        }
    }

    Ok(())
}
