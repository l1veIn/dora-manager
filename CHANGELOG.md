# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-16

### Added

**Core (dm-core)**
- Transpiler: multi-pass pipeline (Parse, Validate, Resolve Paths, Validate Ports, Merge Config, Inject Runtime, Emit) for converting user-friendly YAML to dora-rs native format
- Node Manager: install/uninstall/list nodes with dm.json contract validation
- Runtime Service: start/stop/list/status dataflow runs with lifecycle management
- Event System: structured event logging with SQLite-backed XES-compatible storage
- Environment Manager: DM_HOME directory structure, dora-rs and Python toolchain setup
- Port Schema: type-safe port definitions with validation and auto-inference
- Node Registry (`registry.json`): static mapping of 28 bundled nodes to their sources, embedded at compile time via `include_str!`
- Auto-install: `dm start` automatically imports and installs missing nodes from YAML `source.git` or registry

**CLI (dm-cli)**
- `dm setup` — one-click environment initialization
- `dm install/uninstall/list` — node lifecycle management
- `dm start/stop/runs/logs` — dataflow run management
- `dm start <url>` — download dataflow YAML from URL and start (with progress bar)
- Colorized terminal output with progress indicators

**Server (dm-server)**
- RESTful HTTP API on port 3210
- WebSocket real-time push for run status and logs
- Swagger UI at `/swagger-ui/`
- Single-binary deployment with embedded SvelteKit frontend via rust_embed

**Web UI**
- Visual graph editor (SvelteFlow-based) for dataflow composition
- Runtime workspace with real-time log streaming and panel system
- Responsive widget system for node-specific UI components
- i18n support (Chinese / English)

**Demos**
- `demo-hello-timer` — simplest demo, zero dependencies
- `demo-interactive-widgets` — 4 widget types showcase
- `demo-logic-gate` — AND gate + conditional flow control
- `robotics-object-detection` — webcam + YOLOv8 pipeline

**Node Ecosystem**
- `dora-yolo` — YOLOv8 object detection node with dm.json contract

**Documentation**
- 25-page bilingual project wiki (Chinese + English)
- VitePress documentation site with full navigation
- Architecture overview, API reference, node development guide

**CI/CD**
- GitHub Actions CI: build + test on macOS (aarch64) and Linux (x86_64)
- Release workflow: cross-platform binary builds with GitHub Releases

### Fixed
- Running dataflows showed 0 nodes in Web UI (sync_run_outputs not called for Running state)
- dm-display rendered empty `text []` for null timer ticks (now skips empty content)
