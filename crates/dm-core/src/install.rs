use std::path::PathBuf;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::config;
use crate::types::*;
use crate::util;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

/// Determine the platform asset name patterns to try (in priority order)
fn platform_asset_patterns() -> Vec<&'static str> {
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        vec!["x86_64-apple-darwin"]
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        vec!["aarch64-apple-darwin"]
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        vec!["x86_64-unknown-linux"]
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        vec!["aarch64-unknown-linux"]
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
    )))]
    {
        vec!["unknown-platform"]
    }
}

/// Fetch a specific release or latest from GitHub
async fn fetch_release(client: &Client, version: Option<&str>) -> Result<GithubRelease> {
    let url = match version {
        Some(v) => {
            let tag = if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{v}")
            };
            format!("https://api.github.com/repos/dora-rs/dora/releases/tags/{tag}")
        }
        None => "https://api.github.com/repos/dora-rs/dora/releases/latest".into(),
    };

    let resp = client
        .get(&url)
        .header("User-Agent", "dm/0.1")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API error ({}): {}", status, body);
    }

    let release: GithubRelease = resp.json().await?;
    Ok(release)
}

/// Install a dora version.
/// Progress updates are sent through the optional `progress_tx` channel.
pub async fn install(
    home: &PathBuf,
    version: Option<String>,
    verbose: bool,
    progress_tx: Option<mpsc::UnboundedSender<InstallProgress>>,
) -> Result<InstallResult> {
    let client = Client::new();
    let ver_str = version.as_deref();

    send_progress(&progress_tx, InstallPhase::Fetching, "Fetching release info...");

    let release = fetch_release(&client, ver_str).await?;
    let tag = release.tag_name.trim_start_matches('v').to_string();

    // Check if already installed
    let target_dir = config::versions_dir(home).join(&tag);
    if target_dir.join("dora").exists() {
        return Ok(InstallResult {
            version: tag,
            method: InstallMethod::Binary,
            set_active: false,
        });
    }

    // Try binary download first, fallback to cargo build
    let patterns = platform_asset_patterns();
    let asset = patterns.iter().find_map(|pattern| {
        release.assets.iter().find(|a| {
            a.name.contains(pattern)
                && a.name.contains("dora-cli")
                && (a.name.ends_with(".tar.gz")
                    || a.name.ends_with(".tar.xz")
                    || a.name.ends_with(".zip"))
        })
    });

    let method = match asset {
        Some(asset) => {
            install_from_binary(&client, asset, &tag, &target_dir, verbose, &progress_tx).await?;
            InstallMethod::Binary
        }
        None => {
            send_progress(
                &progress_tx,
                InstallPhase::Building,
                "No binary release for this platform. Building from source...",
            );
            install_from_source(&release.tag_name, &target_dir, verbose).await?;
            InstallMethod::Source
        }
    };

    // Set as active if no active version
    let mut cfg = config::load_config(home)?;
    let set_active = cfg.active_version.is_none();
    if set_active {
        cfg.active_version = Some(tag.clone());
        config::save_config(home, &cfg)?;
    }

    send_progress(
        &progress_tx,
        InstallPhase::Done,
        &format!("dora {} installed successfully.", tag),
    );

    Ok(InstallResult {
        version: tag,
        method,
        set_active,
    })
}

fn send_progress(
    tx: &Option<mpsc::UnboundedSender<InstallProgress>>,
    phase: InstallPhase,
    message: &str,
) {
    if let Some(tx) = tx {
        let _ = tx.send(InstallProgress {
            phase,
            message: message.to_string(),
        });
    }
}

async fn install_from_binary(
    client: &Client,
    asset: &GithubAsset,
    _tag: &str,
    target_dir: &PathBuf,
    verbose: bool,
    progress_tx: &Option<mpsc::UnboundedSender<InstallProgress>>,
) -> Result<()> {
    if verbose {
        eprintln!("[dm] Downloading asset: {}", asset.name);
    }

    send_progress(
        progress_tx,
        InstallPhase::Downloading {
            bytes_done: 0,
            bytes_total: asset.size,
        },
        &format!("Downloading {} ({})", asset.name, util::human_size(asset.size)),
    );

    let resp = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "dm/0.1")
        .send()
        .await?;

    let bytes = {
        let mut buf = Vec::with_capacity(asset.size as usize);
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.extend_from_slice(&chunk);
            send_progress(
                progress_tx,
                InstallPhase::Downloading {
                    bytes_done: buf.len() as u64,
                    bytes_total: asset.size,
                },
                &format!(
                    "Downloading: {}/{}",
                    util::human_size(buf.len() as u64),
                    util::human_size(asset.size)
                ),
            );
        }
        buf
    };

    send_progress(
        progress_tx,
        InstallPhase::Extracting,
        &format!("Extracting to {}...", target_dir.display()),
    );
    std::fs::create_dir_all(target_dir)?;

    if asset.name.ends_with(".tar.gz") || asset.name.ends_with(".tar.xz") {
        extract_tar(&bytes, target_dir)?;
    } else if asset.name.ends_with(".zip") {
        extract_zip(&bytes, target_dir)?;
    }

    // Verify — find the dora binary wherever it ended up
    let dora_bin = target_dir.join("dora");
    if !dora_bin.exists() {
        if let Some(found_bin) = find_dora_binary(target_dir) {
            std::fs::rename(&found_bin, &dora_bin)?;
        } else {
            anyhow::bail!(
                "Could not find dora binary after extraction in {}",
                target_dir.display()
            );
        }
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dora_bin)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dora_bin, perms)?;
    }

    Ok(())
}

/// Install dora by building from source (git clone + cargo build)
async fn install_from_source(
    git_tag: &str,
    target_dir: &PathBuf,
    verbose: bool,
) -> Result<()> {
    if util::check_command("cargo").is_none() {
        anyhow::bail!(
            "No binary release for this platform and Rust is not installed.\n\
             Install Rust first: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        );
    }

    std::fs::create_dir_all(target_dir)?;
    let build_dir = target_dir.join("_build");

    let clone_status = tokio::process::Command::new("git")
        .args([
            "clone",
            "--depth=1",
            "--branch",
            git_tag,
            "https://github.com/dora-rs/dora.git",
            &build_dir.to_string_lossy(),
        ])
        .stdout(if verbose {
            std::process::Stdio::inherit()
        } else {
            std::process::Stdio::piped()
        })
        .stderr(if verbose {
            std::process::Stdio::inherit()
        } else {
            std::process::Stdio::piped()
        })
        .status()
        .await?;

    if !clone_status.success() {
        anyhow::bail!("Failed to clone dora repository at tag {}", git_tag);
    }

    let build_status = tokio::process::Command::new("cargo")
        .args(["build", "--release", "-p", "dora-cli"])
        .current_dir(&build_dir)
        .stdout(if verbose {
            std::process::Stdio::inherit()
        } else {
            std::process::Stdio::piped()
        })
        .stderr(std::process::Stdio::inherit())
        .status()
        .await?;

    if !build_status.success() {
        let _ = std::fs::remove_dir_all(&build_dir);
        anyhow::bail!("cargo build failed for dora-cli");
    }

    // Copy the binary
    let built_bin = build_dir.join("target/release/dora");
    let target_bin = target_dir.join("dora");
    std::fs::copy(&built_bin, &target_bin)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_bin)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_bin, perms)?;
    }

    let _ = std::fs::remove_dir_all(&build_dir);
    Ok(())
}

fn extract_tar(data: &[u8], target_dir: &PathBuf) -> Result<()> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("tar")
        .args(["xzf", "-", "--strip-components=1", "-C"])
        .arg(target_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(data)?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let mut child = Command::new("tar")
            .args(["xzf", "-", "-C"])
            .arg(target_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(data)?;
        }
        let output2 = child.wait_with_output()?;
        if !output2.status.success() {
            let err = String::from_utf8_lossy(&output2.stderr);
            anyhow::bail!("tar extraction failed: {}", err);
        }
    }
    Ok(())
}

fn extract_zip(data: &[u8], target_dir: &PathBuf) -> Result<()> {
    use std::io::Cursor;
    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;
    archive.extract(target_dir)?;
    Ok(())
}

fn find_dora_binary(dir: &PathBuf) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().map(|n| n == "dora").unwrap_or(false) {
                return Some(path);
            }
            if path.is_dir() {
                if path.file_name().map(|n| n == ".venv").unwrap_or(false) {
                    continue;
                }
                if let Some(found) = find_dora_binary(&path) {
                    return Some(found);
                }
            }
        }
    }
    None
}
