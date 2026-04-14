use std::env;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

/// Build common GitHub API headers, attaching an Authorization token
/// when the `GITHUB_TOKEN` environment variable is set.
fn github_headers(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut req = req
        .header("User-Agent", "dm/0.1")
        .header("Accept", "application/vnd.github+json");

    if let Ok(token) = env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
    }

    req
}

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
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        vec!["x86_64-pc-windows"]
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        vec!["aarch64-pc-windows"]
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
    )))]
    {
        vec!["unknown-platform"]
    }
}

pub(super) async fn fetch_release(client: &Client, version: Option<&str>) -> Result<GithubRelease> {
    fetch_release_from_base_url(client, "https://api.github.com", version).await
}

fn release_url(api_base: &str, version: Option<&str>) -> String {
    match version {
        Some(v) => {
            let tag = if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{v}")
            };
            format!("{api_base}/repos/dora-rs/dora/releases/tags/{tag}")
        }
        None => format!("{api_base}/repos/dora-rs/dora/releases/latest"),
    }
}

async fn fetch_release_from_base_url(
    client: &Client,
    api_base: &str,
    version: Option<&str>,
) -> Result<GithubRelease> {
    let url = release_url(api_base, version);

    let resp = github_headers(client.get(&url)).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.as_u16() == 403 || status.as_u16() == 429 {
            anyhow::bail!(
                "GitHub API error ({}): {}\n\n  Hint: You may have hit the API rate limit.\n  Set a GitHub personal access token to increase your limit:\n    export GITHUB_TOKEN=ghp_your_token_here\n\n  Or download the release manually from:\n    https://github.com/dora-rs/dora/releases",
                status, body
            );
        }
        anyhow::bail!("GitHub API error ({}): {}", status, body);
    }

    let release: GithubRelease = resp.json().await?;
    Ok(release)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    use reqwest::Client;

    use super::{fetch_release_from_base_url, release_url};

    #[test]
    fn release_url_defaults_to_latest_endpoint() {
        let url = release_url("https://api.github.test", None);
        assert_eq!(
            url,
            "https://api.github.test/repos/dora-rs/dora/releases/latest"
        );
    }

    #[test]
    fn release_url_normalizes_tag_prefix() {
        assert_eq!(
            release_url("https://api.github.test", Some("0.9.0")),
            "https://api.github.test/repos/dora-rs/dora/releases/tags/v0.9.0"
        );
        assert_eq!(
            release_url("https://api.github.test", Some("v0.9.0")),
            "https://api.github.test/repos/dora-rs/dora/releases/tags/v0.9.0"
        );
    }

    #[tokio::test]
    async fn fetch_release_uses_latest_endpoint_and_parses_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 2048];
            let len = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..len]).into_owned();
            tx.send(request).unwrap();
            let body = r#"{"tag_name":"v0.9.0","assets":[{"name":"dora-cli.zip","browser_download_url":"https://example.invalid/dora-cli.zip","size":42}]}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body.as_bytes()).unwrap();
        });

        let base = format!("http://{addr}");
        let release = fetch_release_from_base_url(&Client::new(), &base, None)
            .await
            .unwrap();
        server.join().unwrap();

        let request = rx.recv().unwrap();
        assert!(request.starts_with("GET /repos/dora-rs/dora/releases/latest "));
        assert_eq!(release.tag_name, "v0.9.0");
        assert_eq!(release.assets.len(), 1);
        assert_eq!(release.assets[0].size, 42);
    }

    #[tokio::test]
    async fn fetch_release_normalizes_version_and_surfaces_api_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 2048];
            let len = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..len]).into_owned();
            tx.send(request).unwrap();
            let body = "not found";
            let header = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body.as_bytes()).unwrap();
        });

        let base = format!("http://{addr}");
        let err = fetch_release_from_base_url(&Client::new(), &base, Some("0.9.0"))
            .await
            .unwrap_err()
            .to_string();
        server.join().unwrap();

        let request = rx.recv().unwrap();
        assert!(request.starts_with("GET /repos/dora-rs/dora/releases/tags/v0.9.0 "));
        assert!(err.contains("GitHub API error (404 Not Found): not found"));
    }
}
