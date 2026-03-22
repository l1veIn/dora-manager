# DM Exclusive Nodes — Design Discussion

> Status: Proposal / Early Exploration

## Background

`dora-rerun` is a dashboard/visualization node that receives data from the dataflow
and renders it in the Rerun desktop application. This works, but has limitations:

- Rerun is a separate desktop application — disconnected from dm
- Requires extra installation
- Local-only (no remote access)
- One-directional (view-only, cannot send data back to the dataflow)

With dm's web UI (and a future Tauri desktop app), we can build **dm-exclusive nodes**
that integrate directly with dm-server via WebSocket, turning the web UI into a
first-class participant in the dataflow.

## Proposed Node Matrix

| Node           | Replaces         | Direction | Core Capability                                 |
|----------------|------------------|-----------|-------------------------------------------------|
| `dm-dashboard` | `dora-rerun`     | DF → Web  | Real-time charts, images, text, 3D point clouds |
| `dm-input`     | `dora-keyboard`  | Web → DF  | Buttons, sliders, text input from the web UI    |
| `dm-logger`    | `terminal-print` | DF → Web  | Structured logs written to events.db             |
| `dm-recorder`  | `dora-record`    | DF → disk | Data recording managed by dm, web UI replay     |
| `dm-config`    | hardcoded env    | Web ↔ DF  | Runtime hot-reload of node parameters            |

## Architecture

### Data Channel Design

dm-exclusive nodes communicate with dm-server through two distinct channels,
chosen based on data characteristics:

```
┌─────────────────────────────────────────────┐
│                  dm-server                  │
│                                             │
│  ┌──────────────┐    ┌──────────────────┐   │
│  │  events.db   │    │    WebSocket     │   │
│  │  (SQLite)    │    │  (real-time)     │   │
│  │              │    │                  │   │
│  │ • ops events │    │ • image frames   │   │
│  │ • lifecycle  │    │ • sensor data    │   │
│  │ • logs       │    │ • text streams   │   │
│  └──────┬───────┘    └────────┬─────────┘   │
│         │                    │              │
│         └────────┬───────────┘              │
│                  ▼                          │
│            Web UI / Tauri                   │
└─────────────────────────────────────────────┘
```

| Module           | Channel      | Rationale                                      |
|------------------|--------------|-------------------------------------------------|
| `dm-logger`      | events.db    | Structured text logs — identical to events model |
| `dm-dashboard`   | WebSocket    | High-frequency binary data cannot go through SQLite |
| `dm-input`       | WebSocket    | Real-time control signals from UI to dataflow   |
| `dm-recorder`    | Direct file  | Large volume recording, not routed through server |

### Priority: dm-dashboard

The highest-value first node is `dm-dashboard`:

```
[dora-yolo] ──image/boxes──→ [dm-dashboard] ──WebSocket──→ [dm-server] ──→ [Web UI]
```

Key advantages over dora-rerun:
- **Remote visualization** — robot runs the dataflow, laptop opens browser to view
- **Zero extra install** — `dm up` provides everything
- **Bidirectional** — web UI controls can feed data back to the dataflow
- **Multi-user** — multiple browsers can observe the same dataflow simultaneously

### dm-input: Bi-directional Control

```
[Web UI slider] ──WebSocket──→ [dm-server] ──→ [dm-input] ──speed: 0.5──→ [motor-controller]
```

This turns dm's web UI into a **robot remote control panel** — users manipulate
controls in the browser and data flows into the running dataflow in real-time.
This is fundamentally impossible with dora-rerun.

## Implementation Notes

- dm-dashboard and dm-input should be Python nodes (matching dora's node ecosystem)
- They connect to dm-server's WebSocket endpoint (to be implemented)
- dm-logger reuses the existing events module directly — no new infrastructure needed
- The WebSocket channel is purely for real-time data; metadata events still go to events.db
