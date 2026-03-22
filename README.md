# dm — Dora Manager

A powerful Rust-based CLI, HTTP API, and Visual Panel for managing [dora-rs](https://github.com/dora-rs/dora) environments. `dm` goes beyond simple version management by providing a Zero-Networking dataflow transpiler, reactive UI widgets, and full runtime orchestration.

## 🎨 Interactive Graph Editor

The centerpiece of Dora Manager is its high-performance, SvelteFlow-based Visual Editor. You can build, visualize, and edit Dora dataflows directly in your browser.

<p align="center">
  <img src="docs/assets/editor_polish_demo.webp" alt="Graph Editor Demo" style="max-width: 100%; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.1)"/>
</p>

- **Right-Click Context Menus**: Seamless node duplication, edge deletion, and quick inspections straight from the canvas workspace.
- **Floating Inspector**: A draggable, resizable window exposing rich configuration schemas dynamically parsed from each Node's capabilities.
- **Real-time Synchronization**: Every edge drawn, duplicate created, and property edited on the visual field binds symmetrically to the underlying YAML model.

### UI Previews

<table align="center">
  <tr>
    <td align="center"><b>Split-View Dataflow Canvas</b></td>
    <td align="center"><b>Deep-Schema Config Inspector</b></td>
  </tr>
  <tr>
    <td align="center"><img src="docs/assets/editor_screenshot.png" width="480"></td>
    <td align="center"><img src="docs/assets/inspector_screenshot.png" width="480"></td>
  </tr>
</table>

<br/>

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

### 1. Build the Suite

Since the SvelteKit frontend is statically embedded directly into the Rust backend server, you must compile the web assets before compiling the Rust crates.

```bash
# Build the SvelteKit Visual Panel
cd web
npm install
npm run build
cd ..

# Build the Rust suite (dm-cli & dm-server)
cargo build --release
```

### 2. Enter the Visual Editor

To spin up the orchestrated API and Visual Panel, simply start the server:

```bash
./target/release/dm-server
```

**Next, open your browser and navigate to: [http://127.0.0.1:3210](http://127.0.0.1:3210) to access the Interactive Graph Editor!**

> 💡 **Tip for Developers**: You can use `./dev.sh` to spin up both the Rust backend and the SvelteKit development server (with Hot Module Replacement) simultaneously.

### 3. Manage Environments (CLI)

You can still use the powerful CLI tool to orchestrate environments silently:

```bash
# Environment lifecycle
./target/release/dm install
./target/release/dm doctor
./target/release/dm use 0.4.1

# Dataflow execution
./target/release/dm up
./target/release/dm start dataflow.yml
./target/release/dm down
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

The Axum REST server binds on `3210` by default.
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

