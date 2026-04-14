use std::path::Path;

use anyhow::Result;

use crate::util;

pub(super) async fn install_from_source(
    git_tag: &str,
    target_dir: &Path,
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

    #[cfg(windows)]
    let built_bin = build_dir.join("target/release/dora.exe");
    #[cfg(not(windows))]
    let built_bin = build_dir.join("target/release/dora");

    #[cfg(windows)]
    let target_bin = target_dir.join("dora.exe");
    #[cfg(not(windows))]
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::test_support::{env_lock, set_path};

    use super::install_from_source;

    fn write_executable(path: &Path, content: &str) {
        fs::write(path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn install_from_source_errors_when_git_clone_fails() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        write_executable(
            &bin_dir.join("cargo"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo cargo 1.0; exit 0; fi\nexit 1\n",
        );
        write_executable(&bin_dir.join("git"), "#!/bin/sh\nexit 1\n");

        let _path = set_path(bin_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(install_from_source(
            "v0.4.1",
            dir.path().join("target").as_path(),
            false,
        ));

        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to clone dora repository at tag v0.4.1"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn install_from_source_errors_when_build_fails_and_cleans_build_dir() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        let target_dir = dir.path().join("target");
        fs::create_dir_all(&bin_dir).unwrap();

        write_executable(
            &bin_dir.join("cargo"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo cargo 1.0; exit 0; fi\nexit 1\n",
        );
        write_executable(
            &bin_dir.join("git"),
            "#!/bin/sh\n/bin/mkdir -p \"$6\"\nexit 0\n",
        );

        let _path = set_path(bin_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(install_from_source("v0.4.1", &target_dir, false));

        let err = result.unwrap_err().to_string();
        assert!(err.contains("cargo build failed for dora-cli"));
        assert!(!target_dir.join("_build").exists());
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn install_from_source_copies_built_binary_and_removes_build_dir() {
        let _guard = env_lock();
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        let target_dir = dir.path().join("target");
        fs::create_dir_all(&bin_dir).unwrap();

        write_executable(
            &bin_dir.join("cargo"),
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo cargo 1.0; exit 0; fi\n/bin/mkdir -p target/release\nprintf '#!/bin/sh\\necho dora\\n' > target/release/dora\n/bin/chmod +x target/release/dora\nexit 0\n",
        );
        write_executable(
            &bin_dir.join("git"),
            "#!/bin/sh\n/bin/mkdir -p \"$6\"\nexit 0\n",
        );

        let _path = set_path(bin_dir.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(install_from_source("v0.4.1", &target_dir, false))
            .unwrap();

        assert!(target_dir.join(crate::config::dora_bin_name()).exists());
        assert!(!target_dir.join("_build").exists());
    }
}
