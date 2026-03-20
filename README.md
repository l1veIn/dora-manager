# dm — Dora Manager

A powerful Rust-based CLI, HTTP API, and Visual Panel for managing [dora-rs](https://github.com/dora-rs/dora) environments. `dm` goes beyond simple version management by providing a Zero-Networking dataflow transpiler, reactive UI widgets, and full runtime orchestration.

## 🚀 Key Features

- **Visual Dataflow Orchestration**: A stunning Svelte/Tailwind web panel with real-time grid layouts, lazy tab loading, and responsive tracking.
- **Smart Reactive Widgets**: Expand `dora-rs` nodes with an immersive arsenal of custom widgets, including sliders, multi-select checkboxes, switches, path selectors (`PathPicker`), and rich media viewers (video via `plyr`, audio, and interactive `JSON` trees).
- **Built-in Node Ecosystem**: Ships with specialized, data-agnostic nodes:
  - `dm-downloader`: HTTP file fetching with SHA/MD5 hashing, automatic zip extraction, and interactive panel bindings.
  - `dm-queue`: High-performance buffer queue with metadata passthrough and idle flush mechanisms.
  - `dm-mjpeg` & `dm-microphone`: Media ingestion fully wired with dynamic control switches.
- **System Health Diagnostics**: Built-in real-time probes (`doctor`) and CPU/Memory usage badges for tracking active run metrics.
- **Zero-Networking Architecture & YAML Transpiler**: Transparently compiles extended Dataflow models on the fly with no underlying network spaghetti.

## 🏗️ Architecture

```text
dm-core   (lib)   → Business logic, Transpiler, Zero-Networking state, Node Installer
dm-cli    (bin)   → CLI & Terminal UI (colored output, progress bars)
dm-server (bin)   → Axum HTTP API & WebSocket Sync (JSON REST on port 3210)
web       (Svelte)→ Reactive visual panel, dynamically rendering widget overrides
```

## ⚡ Quick Start

```bash
# Build the suite
cargo build --release

# Manage your environment
./target/release/dm install
./target/release/dm doctor
./target/release/dm versions
./target/release/dm use 0.4.1

# Control runtime
./target/release/dm up
./target/release/dm down

# Run & view your dataflow in the Visual Panel
./target/release/dm start dataflow.yml

# Pass-through to native dora CLI
./target/release/dm -- stop --name my-dataflow
```

## 📸 Try it out: OpenCV Camera Pipeline

Try out a real-world computer vision dataflow using your webcam in under 30 seconds. This will also boot up a reactive visual panel to monitor the data stream in real-time!

1. **Create `quickstart.yml`**
```yaml
nodes:
  - id: camera
    path: opencv-video-capture
    inputs:
      tick: dora/timer/millis/30
    outputs:
      - image

  - id: plot
    path: opencv-plot
    inputs:
      image: camera/image
```

2. **Run it**
```bash
# Automatically downloads nodes into isolated python venvs, transpiles the graph, and streams your webcam!
# Open the provided web link to view the reactive Panel.
cargo run -- run quickstart.yml
```

## 🔌 HTTP API

Start the Axum server:
```bash
cargo run -p dm-server
```

**Endpoints:**
```bash
curl http://127.0.0.1:3210/api/doctor
curl http://127.0.0.1:3210/api/versions
curl http://127.0.0.1:3210/api/status
curl -X POST http://127.0.0.1:3210/api/install -H 'Content-Type: application/json' -d '{"version":"0.4.1"}'
curl -X POST http://127.0.0.1:3210/api/up
curl -X POST http://127.0.0.1:3210/api/down
```

## ⚙️ Configuration

- **Home directory**: `~/.dm` (override with `--home` flag or `DM_HOME` env var)
- **Config file**: `~/.dm/config.toml`
- **Versions**: `~/.dm/versions/<version>/dora`

## 📦 Install Strategy

1. **Binary download** from GitHub Releases (fastest).
2. **Build from source** via `cargo build` if no binary is available for your platform.
3. Node distribution (`dm-node-install`) uses a `cargo-binstall`-inspired strategy for smooth plugin deployments.

## 📄 License

Apache-2.0

