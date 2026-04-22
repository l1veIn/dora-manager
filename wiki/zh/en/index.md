---
title: Dora Manager Documentation
---

# Dora Manager

Dora Manager (abbreviated as `dm`) is a **dataflow orchestration and management platform** built with Rust. It provides CLI, HTTP API, and visual Web dashboard management capabilities for [dora-rs](https://github.com/dora-rs/dora).

## Quick Navigation

### Getting Started
- [Project Overview](01-project-overview.md) — What Dora Manager is and why it exists
- [Quick Start](02-quickstart.md) — Install, launch, and run your first dataflow
- [Development Environment](03-dev-environment.md) — Build from source and hot-reload workflow

### Core Concepts
- [Node](04-node-concept.md) — dm.json contract and executable units
- [Dataflow](05-dataflow-concept.md) — YAML topology definition and node connections
- [Run](06-run-lifecycle.md) — Lifecycle state machine and metrics tracking
- [Built-in Nodes](07-builtin-nodes.md) — From media capture to AI inference
- [Port Schema](08-port-schema.md) — Port type validation
- [Custom Node Guide](09-custom-node-guide.md) — dm.json complete field reference

### Backend Architecture (Rust)
- [Architecture Overview](10-architecture-overview.md) — dm-core / dm-cli / dm-server layering
- [Transpiler](11-transpiler.md) — Multi-pass pipeline and four-layer config merging
- [Node Management](12-node-management.md) — Installation, import, path resolution, and sandboxing
- [Runtime Service](13-runtime-service.md) — Startup orchestration, status refresh, and CPU/memory metrics
- [Event System](14-event-system.md) — Observability model and XES-compatible event storage
- [HTTP API](15-http-api.md) — REST routes, WebSocket real-time channels, and Swagger docs
- [Configuration](16-config-system.md) — DM_HOME directory structure and config.toml

### Frontend Architecture (Svelte)
- [SvelteKit Structure](17-sveltekit-structure.md) — Route design, API communication, and state management
- [Graph Editor](18-graph-editor.md) — SvelteFlow canvas, context menus, and YAML bidirectional sync
- [Runtime Workspace](19-runtime-workspace.md) — Grid layout, panel system, and real-time log viewer
- [Reactive Widgets](20-reactive-widgets.md) — Widget registry, dynamic rendering, and WebSocket parameter injection
- [i18n and UI](21-i18n-and-ui.md) — Internationalization and UI component library

### Interaction & Bridge
- [Interaction System](22-interaction-system.md) — dm-input / dm-message / Bridge node injection
- [Capability Binding](23-capability-binding.md) — Node capability declaration and runtime role binding
- [Media Streaming](24-media-streaming.md) — MJPEG capture, MediaMTX integration, and WebRTC/HLS distribution

### Engineering
- [Build and Embed](25-build-and-embed.md) — rust-embed static embedding and CI/CD pipeline
- [Testing Strategy](26-testing-strategy.md) — Unit tests, dataflow integration tests, and system test checklists
- [Project Constitution](27-project-constitution.md) — Product vision, decision priorities, and Agent operating rules

## Links

- [GitHub Repository](https://github.com/l1veIn/dora-manager)
- [dora-rs Official Repository](https://github.com/dora-rs/dora)
