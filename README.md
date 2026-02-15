# dm — Dora Manager

A Rust CLI tool and HTTP API for managing [dora-rs](https://github.com/dora-rs/dora) environments: install, switch versions, and monitor runtime status.

## Architecture

```
dm-core   (lib)   → Pure business logic, Serialize/Deserialize types
dm-cli    (bin)   → Terminal UI (colored output, progress bars)
dm-server (bin)   → Axum HTTP API (JSON REST on port 3210)
```

All three crates share `dm-core`. Adding a Tauri frontend later only requires importing `dm-core`.

## Quick Start

```bash
# Build
cargo build --release

# Install latest dora
./target/release/dm install

# Check environment
./target/release/dm doctor

# Show installed & available versions
./target/release/dm versions

# Switch version
./target/release/dm use 0.4.1

# Start/stop runtime
./target/release/dm up
./target/release/dm down

# Pass-through to dora CLI
./target/release/dm -- run dataflow.yml --uv
```

## HTTP API

```bash
# Start server
cargo run -p dm-server

# Endpoints
curl http://127.0.0.1:3210/api/doctor
curl http://127.0.0.1:3210/api/versions
curl http://127.0.0.1:3210/api/status
curl -X POST http://127.0.0.1:3210/api/install -H 'Content-Type: application/json' -d '{"version":"0.4.1"}'
curl -X POST http://127.0.0.1:3210/api/up
curl -X POST http://127.0.0.1:3210/api/down
```

All endpoints return JSON.

## Tests

```bash
cargo test -p dm-core    # 56 tests
```

## Config

- **Home directory**: `~/.dm` (override with `--home` flag or `DM_HOME` env var)
- **Config file**: `~/.dm/config.toml`
- **Versions**: `~/.dm/versions/<version>/dora`

## Install Strategy

1. **Binary download** from GitHub Releases (fastest)
2. **Build from source** via `cargo build` if no binary available for your platform

## License

Apache-2.0
