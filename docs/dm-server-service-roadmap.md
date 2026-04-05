# dm-server Service Roadmap

> Status: **Future design notes**

## Architecture Vision

dm-server is the **single gateway** between dora nodes and the web frontend.
All node-to-web communication flows through dm-server.

```
┌─────────────────────────────────────────────────┐
│  dm-server                                      │
│                                                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐│
│  │  Run     │ │  Log     │ │  Artifact        ││
│  │  Manager │ │  Service │ │  Server          ││
│  │  ✅ done │ │  ✅ done │ │  ✅ done         ││
│  └──────────┘ └──────────┘ └──────────────────┘│
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐│
│  │  Message │ │  Media   │ │  Recording       ││
│  │  Service │ │  Stream  │ │  Service         ││
│  │  🔜 next │ │  📋 plan │ │  📋 plan         ││
│  └──────────┘ └──────────┘ └──────────────────┘│
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐│
│  │  Metrics │ │  State   │ │  Node            ││
│  │  Service │ │  Store   │ │  Lifecycle       ││
│  │  💡 idea │ │  💡 idea │ │  💡 idea         ││
│  └──────────┘ └──────────┘ └──────────────────┘│
└─────────────────────────────────────────────────┘
```

---

## Existing Services (✅)

### Run Manager
- Start / stop / list runs
- Dataflow transpilation
- Run metadata and status tracking

### Log Service
- File-based log capture (stdout/stderr per node)
- WebSocket push via file system notify
- Terminal Panel data source

### Artifact Server
- Serve files from run output directory
- Used by Message Panel for images, audio, video

---

## In Progress (🔜)

### Message Service
- Lightweight JSON message relay + persistence
- Unified emit / on model with tag-based routing
- See: [workspace-message-panel-design.md](./workspace-message-panel-design.md)

---

## Planned (📋)

### Media Stream Service
- Binary frame relay (MJPEG, WebSocket binary)
- Node pushes encoded frames via WS → server broadcasts to web clients
- Endpoint: `GET /api/runs/{id}/streams/{node_id}`
- Nodes register streams via Message (`tag: "stream"`)
- Panel: `PanelLive` — renders `<img>` for MJPEG or `<canvas>` for WS binary
- Replaces dm-mjpeg's standalone HTTP server

### Recording Service
- Parquet-based Arrow event capture at the transport layer
- Start/stop/status API
- Event query API for frontend replay
- See: [run-recording-replay.md](./run-recording-replay.md)

---

## Ideas (💡)

### Metrics Service
Node-level performance telemetry.

- **Data**: CPU, memory, throughput (msg/s), latency (ms), queue depth
- **Collection**: nodes push metrics via Message (`tag: "metrics"`)
  or dm-server collects from dora daemon API
- **Storage**: time-series in SQLite (or future InfluxDB/Prometheus)
- **Panel**: `PanelMetrics` — sparkline charts, gauges, health indicators
- **Use case**: identify bottleneck nodes, monitor resource usage

### State Store (KV)
A run-scoped key-value store for cross-node shared state.

- **API**: `PUT /runs/{id}/state/{key}`, `GET /runs/{id}/state/{key}`
- **Use case**: a configuration node sets `model_name = "qwen-7b"`,
  other nodes read it at startup
- **Use case**: a coordinator node writes `phase = "inference"`,
  UI Panel shows current pipeline phase
- **Difference from Message**: Message is event-driven (append-only stream),
  State is a mutable dictionary (latest value wins)
- **Note**: `message_snapshots` already partially serves this purpose.
  Evaluate whether a dedicated KV API adds enough value over using
  Message with specific tags.

### Node Lifecycle Service
Runtime control of individual nodes.

- **Actions**: restart node, hot-reload config, pause/resume
- **API**: `POST /runs/{id}/nodes/{node_id}/restart`
- **Dependency**: requires dora runtime API support for per-node control
- **Panel**: node status indicators in RuntimeGraphView (green/yellow/red)

### Debug / Inspection Service
Inspect Arrow data flowing between nodes in real-time.

- **Mode**: "tap" an edge in the dataflow graph → see data samples
- **Implementation**: inject a transparent proxy node (like Recording)
  that samples and relays data previews to the web
- **Panel**: `PanelInspector` — shows decoded Arrow data as table/JSON
- **Use case**: debugging data transformations without adding display nodes

### Audio Stream Service
Same architecture as Media Stream, but for audio.

- **Protocol**: WebSocket binary (PCM/WAV chunks) or WebAudio API
- **Nodes**: dm-microphone, dora-kokoro-tts already produce audio
- **Panel**: `PanelAudio` — waveform visualization + playback controls
- **Shared infra**: could share the same streaming endpoint as video,
  differentiated by MIME type

---

## Service vs Node: When to Use Which?

A recurring design question: should a capability live in dm-server
(as a service) or in a dora node?

| Put in dm-server when... | Put in a node when... |
|---|---|
| It's a **platform concern** (all runs need it) | It's a **user concern** (opt-in per dataflow) |
| It needs to **bridge node ↔ web** | It operates **within the Arrow data flow** |
| It needs **centralized state** (DB, broadcast) | It's **stateless** or self-contained |
| Multiple nodes share the resource | Only one node uses it |

**Examples:**
- Message relay → server (all UI interaction flows through it)
- JPEG encoding → node (data processing belongs in the dataflow)
- Run metrics → server (aggregates across all nodes)
- TTS inference → node (compute-intensive, specific to one use case)
