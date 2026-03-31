# DM Architecture Principles

> Established: 2026-03-31

These principles emerged from deep analysis of the dm-panel subsystem and inform all future architectural decisions for DM.

---

## 1. Node Business Purity

**A node should do exactly one thing.**

A node is either a computation unit, a storage unit, or an interaction unit. It should never mix these concerns. If a node starts doing two things, it should be split into two nodes.

Bad:
```
dm-panel: receives data + stores data + renders UI + handles input
```

Good:
```
dm-store:            receives data → serializes → writes to disk
dm-panel-upstream:   receives data → sends to server for display
dm-panel-downstream: receives user input from server → outputs Arrow
```

---

## 2. Node Family Classification

All nodes belong to one of four families based on their single responsibility:

| Family | Responsibility | Examples |
|--------|---------------|----------|
| **Compute** | Data transformation | dm-llm, dm-whisper, dm-tts |
| **Storage** | Data persistence (serialize + write) | dm-store (future) |
| **Interaction** | Human I/O interface | dm-panel, dm-keyboard, dm-mic |
| **Source** | Data generation / event emission | dm-timer, dm-file-reader |

A node's family determines its architectural constraints:
- **Compute** nodes have no side effects beyond their Arrow outputs
- **Storage** nodes write to the filesystem but don't render
- **Interaction** nodes bridge human and dataflow (display + controls)
- **Source** nodes produce data but don't consume node outputs

---

## 3. dm-core Must Be Node-Agnostic

**The core engine must not contain knowledge of any specific node.**

dm-core's job is:
- Manage dataflow lifecycle (start, stop, status)
- Transpile DM YAML → dora YAML (resolve paths, merge config)
- Route data between nodes via dora runtime

dm-core must NOT:
- Hardcode any node IDs (no `RESERVED_NODE_IDS`)
- Have special enum variants for specific nodes (no `DmNode::Panel`)
- Store node-specific metadata in the run model (no `has_panel`)
- Contain node-specific business logic (no `PanelStore`)

If a node needs special framework support, that support belongs in the application layer (dm-server, dm-cli), not in the core.

---

## 4. UI Is a Node-Level Concern, Not a Per-Node Feature

**"Every node should have its own UI" is a false requirement.** It violates node business purity.

Instead:
- UI is handled by dedicated **interaction family** nodes
- Computation nodes output Arrow data and nothing else — they don't know or care how their output is displayed
- The platform's monitoring layer (RuntimeGraphView, NodeInspector) observes node status, but this is separate from node UI

---

## 5. Display and Persistence Are Orthogonal

The display path should leverage persisted artifacts, not reinvent them:

```
Compute node → dm-store → filesystem → Interaction node (reads & renders)
```

Or for real-time display, Arrow signals flow directly:

```
Compute node → Interaction node (renders Arrow data)
```

The interaction node does NOT store data. Storage is the storage node's job.

---

## 6. Interaction Nodes Are Platform-Agnostic

An interaction node's logic (what to display, what input to accept) is platform-independent. The same node should work across:

- **Web** (SvelteKit via dm-server)
- **CLI** (stdin/stdout)
- **Mobile** (native app / PWA)
- **Desktop** (Electron / Tauri)

The rendering is handled by platform adapters. The dataflow YAML remains unchanged across platforms.

---

## 7. Panel IPC Architecture (Future Direction)

The current monolithic dm-panel will be replaced by a Tauri/Electron-style IPC architecture:

```
Upstream node (in dataflow) → dm-server (IPC relay) → Web frontend (render)
Web frontend (user action)  → dm-server (IPC relay) → Downstream node (in dataflow)
```

- **dm-server** becomes the IPC bridge between dataflow nodes and the web UI
- **Upstream/downstream** are standard dora nodes with no special treatment
- **dm-core** knows nothing about this — it's pure application-layer orchestration

---

## Application

These principles should be validated against any new feature or architecture proposal:

1. Does this feature make a node do more than one thing? → Split it
2. Does this change require dm-core to know about a specific node? → Push it to app layer
3. Does this add UI logic to a computation node? → Create a dedicated interaction node
4. Does this mix storage with display? → Separate them
