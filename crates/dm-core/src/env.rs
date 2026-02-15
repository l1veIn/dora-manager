use crate::types::EnvItem;
use crate::util;

/// Check Python availability (looks for python3, python3.11, python)
pub async fn check_python() -> EnvItem {
    for cmd in &["python3.11", "python3", "python"] {
        if let Some(path) = util::check_command(cmd) {
            let ver = util::get_command_version(cmd, &["--version"])
                .await
                .unwrap_or_default();
            return EnvItem {
                name: "Python".into(),
                found: true,
                path: Some(path),
                version: Some(ver),
                suggestion: None,
            };
        }
    }
    EnvItem {
        name: "Python".into(),
        found: false,
        path: None,
        version: None,
        suggestion: Some("Install Python 3.11+: https://www.python.org/downloads/".into()),
    }
}

/// Check uv availability
pub async fn check_uv() -> EnvItem {
    if let Some(path) = util::check_command("uv") {
        let ver = util::get_command_version("uv", &["--version"])
            .await
            .unwrap_or_default();
        EnvItem {
            name: "uv".into(),
            found: true,
            path: Some(path),
            version: Some(ver),
            suggestion: None,
        }
    } else {
        EnvItem {
            name: "uv".into(),
            found: false,
            path: None,
            version: None,
            suggestion: Some("Install uv: pip install uv".into()),
        }
    }
}

/// Check Rust / cargo (optional, for Rust nodes)
pub async fn check_rust() -> EnvItem {
    if let Some(path) = util::check_command("cargo") {
        let ver = util::get_command_version("cargo", &["--version"])
            .await
            .unwrap_or_default();
        EnvItem {
            name: "Rust".into(),
            found: true,
            path: Some(path),
            version: Some(ver),
            suggestion: None,
        }
    } else {
        EnvItem {
            name: "Rust".into(),
            found: false,
            path: None,
            version: None,
            suggestion: Some("Optional. Install: https://rustup.rs".into()),
        }
    }
}
