---
title: Dora Manager Documentation
---

# Dora Manager

Dora Manager (abbreviated as `dm`) is a **dataflow orchestration and management platform** built with Rust. It provides CLI, HTTP API, and visual Web dashboard management capabilities for [dora-rs](https://github.com/dora-rs/dora).

## Quick Navigation

### Getting Started
- [Project Overview](01-project-overview.md) — What Dora Manager is and why it exists
- [Quick Start](02-quickstart.md) — Build, launch, and run your first dataflow
- [Development Environment](03-dev-environment.md) — Dev environment setup and hot-reload workflow

### Core Concepts
- [Node](04-node-concept.md) — dm.json contract and executable units
- [Dataflow](05-dataflow-concept.md) — YAML topology and node connections
- [Run](06-run-lifecycle.md) — Lifecycle, state, and metrics tracking

### Backend Architecture (Rust)
- [Architecture Overview](07-architecture-overview.md) — dm-core / dm-cli / dm-server layering
- [Transpiler](08-transpiler.md) — Multi-pass pipeline and four-layer config merging
- [Node Management](09-node-management.md) — Installation, import, path resolution, and sandboxing
- [Runtime Service](10-runtime-service.md) — Startup orchestration, status refresh, and metrics
- [Event System](11-event-system.md) — Observability model and XES-compatible storage
- [HTTP API](12-http-api.md) — Route overview and Swagger documentation
- [Configuration](13-config-system.md) — DM_HOME directory structure and config.toml

### Frontend Architecture (Svelte)
- [SvelteKit Structure](14-sveltekit-structure.md) — Project structure and API communication layer
- [Graph Editor](15-graph-editor.md) — SvelteFlow canvas and YAML synchronization
- [Runtime Workspace](16-runtime-workspace.md) — Grid layout, panel system, and real-time interaction
- [Reactive Widgets](17-reactive-widgets.md) — Control registry and dynamic rendering
- [i18n and UI](18-i18n-and-ui.md) — Internationalization and UI component library

### Node Ecosystem
- [Built-in Nodes](19-builtin-nodes.md) — From media capture to AI inference
- [Port Schema](20-port-schema.md) — Arrow type system and port validation
- [Interaction System](21-interaction-system.md) — dm-input / dm-display / WebSocket messaging
- [Custom Node Guide](22-custom-node-guide.md) — dm.json complete field reference

### Engineering
- [Build and Embed](23-build-and-embed.md) — rust_embed static embedding and release workflow
- [CI/CD](24-ci-cd.md) — GitHub Actions build and release configuration
- [Testing Strategy](25-testing-strategy.md) — Dataflow integration testing strategy and checklists

## Links

- [GitHub Repository](https://github.com/l1veIn/dora-manager)
- [dora-rs Official Repository](https://github.com/dora-rs/dora)
