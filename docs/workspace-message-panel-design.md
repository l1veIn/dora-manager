# Workspace Message & Panel Architecture

> Status: **Design document (approved for implementation)**

## Overview

This document defines the unified architecture for node–web interaction in DM.
It replaces the current ad-hoc stream/input subsystems with two cleanly
separated concepts:

- **Message** — the universal communication and persistence unit
- **Panel** — the frontend visualization layer

### Core Insight

**Everything is a message.** Node data pushes, widget registrations, and user
actions are all modeled as the same `Message` type. The `tag` field provides
semantic differentiation.

### Design Principles

1. **One concept** — Message is the only data unit.
2. **Two operations** — `emit` and `on`.
3. **Nodes don't know about Panels** — nodes only emit and on messages.
4. **Server is a dumb relay** — receive, persist, broadcast. No type-specific logic.
5. **Tag for semantics** — `tag` distinguishes data types; Panels filter by tag.
6. **Persistence by default** — so completed runs remain viewable.

---

## Message Model

### Data Structure

```typescript
interface Message {
    seq: number;           // Auto-increment (server-assigned)
    from: string;          // Source ("node-id" or "web")
    tag: string;           // Semantic type
    payload: any;          // Arbitrary JSON
    timestamp: number;     // Unix seconds
}
```

### Tag Conventions

Tags are open-ended strings. Recommended conventions:

| Tag | From | Payload example | Purpose |
|-----|------|-----------------|---------|
| `text` | dm-display | `{ "content": "hello" }` | Plain text display |
| `image` | dm-display | `{ "file": "output.png" }` | Image artifact |
| `json` | dm-display | `{ "content": {...} }` | JSON tree |
| `markdown` | dm-display | `{ "content": "# Title" }` | Rich text |
| `audio` | dm-display | `{ "file": "speech.wav" }` | Audio player |
| `video` | dm-display | `{ "file": "clip.mp4" }` | Video player |
| `chart` | dm-charts | `{ "type": "line", "labels": [], "series": [] }` | Chart render |
| `table` | dm-table | `{ "columns": [], "rows": [] }` | Data table |
| `widgets` | input nodes | `{ "value": { "type": "textarea", ... } }` | Widget registration |
| `input` | web | `{ "to": "prompt", "output_id": "value", "value": "..." }` | User action |

New node types can define their own tags freely.

---

## Node-Side API

### Static Declaration (dm.json)

Nodes optionally declare interaction capabilities:

```json
{
  "id": "dm-display",
  "interaction": {
    "emit": ["text", "image", "json", "markdown", "audio", "video"]
  }
}
```

```json
{
  "id": "dm-text-input",
  "interaction": {
    "emit": ["widgets"],
    "on": true
  }
}
```

This is metadata only — used for auto-creating default Panel layouts and
editor tooling. Actual behavior is determined at runtime.

### Runtime API

Two operations:

```python
# emit — send a message (HTTP POST)
def emit(tag: str, payload: dict):
    requests.post(f"{server}/api/runs/{run_id}/messages", json={
        "from": node_id,
        "tag": tag,
        "payload": payload,
    })

# on — receive messages (WebSocket)
ws = connect(f"{ws_url}/api/runs/{run_id}/messages/ws/{node_id}")
for raw in ws:
    msg = json.loads(raw)
    handle(msg)
```

### Node Examples

**dm-display** (emit only):

```python
# Emit text content
emit("text", { "content": "Hello world", "label": "Display" })

# Emit image artifact
emit("image", { "file": "frames/output.png", "label": "Camera" })
```

**dm-text-input** (emit + on):

```python
# On startup: register widgets
emit("widgets", {
    "value": { "type": "textarea", "label": "Prompt", "placeholder": "Type..." }
})

# Listen for user actions
def on_message(msg):
    if msg["tag"] == "input" and msg["payload"]["to"] == node_id:
        output_id = msg["payload"]["output_id"]
        value = msg["payload"]["value"]
        node.send_output(output_id, value)
```

**dm-charts** (emit only):

```python
# Emit chart data
emit("chart", {
    "type": "line",
    "labels": [1, 2, 3, 4, 5],
    "series": [{ "name": "Temperature", "data": [22, 23, 21, 24, 22] }]
})
```

**Dynamic widget registration** (runtime flexibility):

```python
# Model node: add a button after model is loaded
def on_input(self, input_id, value, metadata):
    if input_id == "model_ready":
        emit("widgets", {
            "generate": { "type": "button", "label": "🚀 Generate" }
        })
```

---

## Server Architecture

### Single Responsibility

The server does exactly one thing for all messages:

```
Receive → Persist → Broadcast
```

No type-specific logic. No awareness of charts vs widgets vs actions.

### Storage

Two SQLite tables in the per-run `interaction.db`:

```sql
-- All messages, append-only
CREATE TABLE messages (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id     TEXT NOT NULL,          -- "from" field
    tag         TEXT NOT NULL,
    payload     TEXT NOT NULL,          -- JSON
    timestamp   INTEGER NOT NULL
);
CREATE INDEX idx_messages_node_tag ON messages(node_id, tag, seq);

-- Latest state per (from, tag) for quick resume
CREATE TABLE message_snapshots (
    node_id     TEXT NOT NULL,
    tag         TEXT NOT NULL,
    payload     TEXT NOT NULL,          -- JSON
    seq         INTEGER NOT NULL,       -- seq of latest message
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (node_id, tag)
);
```

On every message received:

1. `INSERT INTO messages ...` → get seq
2. `INSERT OR REPLACE INTO message_snapshots ...` → upsert latest
3. Broadcast notification via WebSocket

### API

```
POST   /runs/{id}/messages             ← emit (nodes and web both use this)
GET    /runs/{id}/messages             ← query history (paginated)
GET    /runs/{id}/messages/snapshots   ← latest state per (node, tag)
WS     /runs/{id}/messages/ws          ← subscribe to all notifications (web panels)
WS     /runs/{id}/messages/ws/{node}   ← subscribe filtered to a node (node listen)
GET    /runs/{id}/artifacts/{path}     ← serve artifact files (unchanged)
```

Query params for `GET /messages`:

- `after_seq` / `before_seq` — pagination
- `from` — filter by source node
- `tag` — filter by tag
- `limit` — max results (default 200)
- `desc` — reverse order

### Broadcast

Single `broadcast::Sender<MessageNotification>` in AppState:

```rust
pub struct AppState {
    pub home: Arc<PathBuf>,
    pub events: Arc<EventStore>,
    pub messages: broadcast::Sender<MessageNotification>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageNotification {
    pub run_id: String,
    pub seq: i64,
    pub from: String,
    pub tag: String,
}
```

WebSocket handlers filter notifications by run_id (and optionally node_id
for per-node sockets) before forwarding to clients.

---

## Frontend Panel Model

### Panel Types

```typescript
type PanelKind = "message" | "input" | "chart" | "table" | "terminal";
```

Each PanelKind corresponds to a Svelte component that knows how to render
a specific class of messages.

### Panel Subscription

A Panel configures which messages it cares about:

```typescript
interface PanelConfig {
    nodes: string[];      // ["dm-display"] or ["*"] for all
    tags: string[];       // ["text", "image"] or ["*"] for all
    [key: string]: any;   // kind-specific settings
}
```

Examples:

| Panel | nodes | tags | Behavior |
|-------|-------|------|----------|
| Message Feed | `["*"]` | `["text","image","json","markdown","audio","video"]` | Shows all display messages |
| Input Board | `["*"]` | `["widgets"]` | Reads widget registrations, renders controls |
| Chart | `["dm-charts"]` | `["chart"]` | Renders chart from latest snapshot |
| Chat | `["user", "assistant"]` | `["chat"]` | Multi-node conversation view |
| Terminal | N/A | N/A | Log files (unchanged, not via messages) |

### How Panels Get Data

**On mount:** fetch snapshots from `GET /messages/snapshots` — restores
latest state immediately (handles page refresh).

**For history panels (message feed):** also fetch `GET /messages?desc=true&limit=50`
for scrollable history.

**Live updates:** subscribe via WebSocket, fetch new messages on notification.

### How Input Panel Works

1. Input Panel subscribes to `tag: "widgets"` snapshots
2. Each snapshot with `tag: "widgets"` contains a node's widget schema
3. Panel groups widgets by source node_id, renders controls
4. User interacts → Panel calls `POST /messages` with:
   ```json
   { "from": "web", "tag": "input", "payload": { "to": "prompt", "output_id": "value", "value": "hello" } }
   ```
5. Node's WebSocket receives this message and processes it

### Panel Creation

**Auto-create:** on run start, scan dataflow nodes' dm.json `interaction`
fields to generate a default layout.

**Manual:** user clicks "⊕" to add panels and configure subscriptions.

### Terminal Panel

Terminal remains a special system-level panel. It reads log files via the
existing `run_ws` mechanism, not through messages. Reasons:

- Data source is log files, not node emits
- Volume is orders of magnitude higher
- Needs per-node switching within a single panel
- Existing implementation works well

---

## Persistence & Run Lifecycle

### During Run (Live)

- Messages stream in real-time via WebSocket
- All messages persisted to `messages` table
- Snapshots updated on every message

### Page Refresh During Run

- Panels restore from snapshots (latest state)
- Message Feed panel fetches recent history
- WebSocket reconnects for live updates

### After Run (Completed)

- Same page, same panels — data comes from SQLite instead of WebSocket
- Message Feed shows full history (scrollable)
- Input Panel shows last known widget state (read-only, controls disabled)
- Chart/Table panels show final snapshot

### Relationship to Future Recording

```
Arrow layer (node ↔ node):    Parquet recording (future)
Message layer (node ↔ web):   SQLite (this design)
Log layer (stdout/stderr):    File system (existing)
```

When Arrow Recording is implemented, messages become less critical because
all node I/O can be replayed. Message Store remains useful for lightweight
inspection without full replay.

---

## Mapping to Current Implementation

| Current | New |
|---------|-----|
| `POST /interaction/stream` | `POST /messages` (tag = render type) |
| `GET /interaction/stream/messages` | `GET /messages` |
| `GET /interaction` (sources + inputs) | `GET /messages/snapshots` |
| `POST /interaction/input/register` | `POST /messages` (tag = "widgets") |
| `POST /interaction/input/events` | `POST /messages` (tag = "input", from = "web") |
| `GET /interaction/input/claim/{node}` | `WS /messages/ws/{node}` |
| `WS /interaction/ws` | `WS /messages/ws` |
| `WS /interaction/input/ws/{node}` | `WS /messages/ws/{node}` |
| `InteractionNotification` | `MessageNotification` |
| `InputEventNotification` | `MessageNotification` |
| `StreamService` | `MessageService` |
| `InputService` | `MessageService` |
| `stream_messages` table | `messages` table |
| `stream_sources` table | `message_snapshots` table |
| `input_bindings` table | `message_snapshots` (tag = "widgets") |
| `input_events` table | `messages` (tag = "input") |
| `DisplayStream` component | `PanelMessage` component |
| `InputBoard` component | `PanelInput` component |
| `WidgetTerminal` component | Unchanged |
