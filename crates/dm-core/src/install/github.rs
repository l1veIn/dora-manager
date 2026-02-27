use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct GithubRelease {
    pub tag_name: String,
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

pub(super) fn platform_asset_patterns() -> Vec<&'static str> {
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

pub(super) async fn fetch_release(client: &Client, version: Option<&str>) -> Result<GithubRelease> {
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
