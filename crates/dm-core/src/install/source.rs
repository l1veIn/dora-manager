use std::path::Path;

use anyhow::Result;

use crate::util;

pub(super) async fn install_from_source(git_tag: &str, target_dir: &Path, verbose: bool) -> Result<()> {
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
