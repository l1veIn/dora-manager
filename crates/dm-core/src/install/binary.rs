use std::path::Path;

use anyhow::Result;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::types::{InstallPhase, InstallProgress};
use crate::util;

use super::archive::{extract_tar, extract_zip, find_dora_binary};
use super::github::GithubAsset;
use super::progress::send_progress;

pub(super) async fn install_from_binary(
    client: &Client,
    asset: &GithubAsset,
    target_dir: &Path,
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
        &format!(
            "Downloading {} ({})",
            asset.name,
            util::human_size(asset.size)
        ),
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dora_bin)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dora_bin, perms)?;
    }

    Ok(())
}
