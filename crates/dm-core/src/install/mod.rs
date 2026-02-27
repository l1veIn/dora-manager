mod archive;
mod binary;
mod github;
mod progress;
mod source;

use std::path::Path;

use anyhow::Result;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::config;
use crate::types::*;

/// Install a dora version.
/// Progress updates are sent through the optional `progress_tx` channel.
pub async fn install(
    home: &Path,
    version: Option<String>,
    verbose: bool,
    progress_tx: Option<mpsc::UnboundedSender<InstallProgress>>,
) -> Result<InstallResult> {
    let client = Client::new();
    let ver_str = version.as_deref();

    progress::send_progress(
        &progress_tx,
        InstallPhase::Fetching,
        "Fetching release info...",
    );

    let release = github::fetch_release(&client, ver_str).await?;
    let tag = release.tag_name.trim_start_matches('v').to_string();

    let target_dir = config::versions_dir(home).join(&tag);
    if target_dir.join("dora").exists() {
        return Ok(InstallResult {
            version: tag,
            method: InstallMethod::Binary,
            set_active: false,
        });
    }

    let patterns = github::platform_asset_patterns();
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
            binary::install_from_binary(&client, asset, &target_dir, verbose, &progress_tx).await?;
            InstallMethod::Binary
        }
        None => {
            progress::send_progress(
                &progress_tx,
                InstallPhase::Building,
                "No binary release for this platform. Building from source...",
            );
            source::install_from_source(&release.tag_name, &target_dir, verbose).await?;
            InstallMethod::Source
        }
    };

    let mut cfg = config::load_config(home)?;
    let set_active = cfg.active_version.is_none();
    if set_active {
        cfg.active_version = Some(tag.clone());
        config::save_config(home, &cfg)?;
    }

    progress::send_progress(
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::sync::{Mutex, OnceLock};

    use tempfile::tempdir;
    use tokio::sync::mpsc;

    use super::*;

    #[test]
    fn platform_asset_patterns_is_not_empty() {
        assert!(!github::platform_asset_patterns().is_empty());
    }

    #[test]
    fn send_progress_emits_message_when_channel_exists() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        progress::send_progress(&Some(tx), InstallPhase::Fetching, "Fetching release info...");

        let msg = rx.try_recv().unwrap();
        assert!(matches!(msg.phase, InstallPhase::Fetching));
        assert_eq!(msg.message, "Fetching release info...");
    }

    #[test]
    fn extract_zip_rejects_invalid_data() {
        let dir = tempdir().unwrap();

        let err = archive::extract_zip(b"not-a-zip", dir.path()).unwrap_err().to_string();
        assert!(!err.is_empty());
    }

    #[test]
    fn extract_tar_rejects_invalid_data() {
        let dir = tempdir().unwrap();

        let err = archive::extract_tar(b"not-a-tar", dir.path()).unwrap_err().to_string();
        assert!(err.contains("tar extraction failed"));
    }

    #[test]
    fn find_dora_binary_finds_nested_binary_and_skips_venv() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let venv_bin = root.join(".venv").join("bin");
        fs::create_dir_all(&venv_bin).unwrap();
        fs::write(venv_bin.join("dora"), "ignored").unwrap();

        let nested = root.join("nested").join("bin");
        fs::create_dir_all(&nested).unwrap();
        let dora_path = nested.join("dora");
        fs::write(&dora_path, "real").unwrap();

        let found = archive::find_dora_binary(root).unwrap();
        assert_eq!(found, dora_path);
    }

    #[tokio::test]
    async fn install_from_binary_extracts_zip_and_places_dora() {
        let dir = tempdir().unwrap();
        let target_dir = dir.path().join("install-target");

        let zip_bytes = {
            let mut cursor = std::io::Cursor::new(Vec::new());
            {
                let mut zip = zip::ZipWriter::new(&mut cursor);
                let opts: zip::write::SimpleFileOptions = zip::write::FileOptions::default();
                zip.start_file("nested/dora", opts).unwrap();
                zip.write_all(b"binary").unwrap();
                zip.finish().unwrap();
            }
            cursor.into_inner()
        };

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = zip_bytes.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/zip\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        });

        let asset = github::GithubAsset {
            name: "dora-cli-test.zip".to_string(),
            browser_download_url: format!("http://{}/download.zip", addr),
            size: zip_bytes.len() as u64,
        };

        binary::install_from_binary(&reqwest::Client::new(), &asset, &target_dir, false, &None)
            .await
            .unwrap();
        server.join().unwrap();

        assert!(target_dir.join("dora").exists());
    }

    #[test]
    fn install_from_source_errors_when_cargo_is_unavailable() {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        let dir = tempdir().unwrap();
        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(source::install_from_source("v0.4.1", dir.path(), false));

        if let Some(path) = original_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }

        let err = result.unwrap_err().to_string();
        assert!(err.contains("Rust is not installed"));
    }
}
