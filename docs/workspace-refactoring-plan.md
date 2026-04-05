# Workspace Refactoring Plan

> Status: **Ready for implementation**
> Prereq: Read [workspace-message-panel-design.md](./workspace-message-panel-design.md) first.

## Scope

Refactor the current stream/input subsystems into the unified Message + Panel
architecture. Terminal panel is out of scope (remains unchanged).

---

## Phase 1: Backend — MessageService

### Step 1.1: Create `services/message.rs`

Replaces both `services/stream.rs` and `services/input.rs` with a single
unified service.

**Tables:**

```sql
CREATE TABLE messages (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id   TEXT NOT NULL,
    tag       TEXT NOT NULL,
    payload   TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE INDEX idx_messages_node_tag ON messages(node_id, tag, seq);

CREATE TABLE message_snapshots (
    node_id    TEXT NOT NULL,
    tag        TEXT NOT NULL,
    payload    TEXT NOT NULL,
    seq        INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (node_id, tag)
);
```

**Public API:**

```rust
impl MessageService {
    pub fn open(home: &Path, run_id: &str) -> Result<Self>;
    pub fn push(&self, from: &str, tag: &str, payload: &Value, ts: i64) -> Result<i64>;
    pub fn list(&self, filter: &MessageFilter) -> Result<MessagesResponse>;
    pub fn snapshots(&self) -> Result<Vec<MessageSnapshot>>;
}
```

`push()` does INSERT into messages + UPSERT into snapshots in one transaction.

### Step 1.2: Simplify `state.rs`

```rust
// Before: two broadcast channels
pub interaction_events: broadcast::Sender<InteractionNotification>,
pub input_events: broadcast::Sender<InputEventNotification>,

// After: one broadcast channel
pub messages: broadcast::Sender<MessageNotification>,
```

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MessageNotification {
    pub run_id: String,
    pub seq: i64,
    pub from: String,
    pub tag: String,
}
```

### Step 1.3: Create `handlers/messages.rs`

Replaces `handlers/interaction.rs`.

**Handlers:**

| Handler | Route | Notes |
|---------|-------|-------|
| `push_message` | `POST /runs/{id}/messages` | From nodes and web |
| `list_messages` | `GET /runs/{id}/messages` | Paginated, filtered by from/tag |
| `get_snapshots` | `GET /runs/{id}/messages/snapshots` | Latest per (from, tag) |
| `messages_ws` | `WS /runs/{id}/messages/ws` | Web panel subscription |
| `node_ws` | `WS /runs/{id}/messages/ws/{node_id}` | Node-targeted subscription |
| `get_interaction` | `GET /runs/{id}/interaction` | Compat: returns snapshots summary |
| `serve_artifact_file` | `GET /runs/{id}/artifacts/{path}` | Unchanged, move from old file |

Request type:

```rust
#[derive(Deserialize)]
pub struct PushMessageRequest {
    pub from: String,
    pub tag: String,
    pub payload: serde_json::Value,
    pub timestamp: Option<i64>,
}
```

### Step 1.4: Update `main.rs`

Replace interaction routes with message routes:

```rust
// Before
.route("/api/runs/{id}/interaction", get(handlers::get_interaction))
.route("/api/runs/{id}/interaction/ws", get(handlers::interaction_ws))
.route("/api/runs/{id}/interaction/stream", post(handlers::post_stream))
.route("/api/runs/{id}/interaction/stream/messages", get(handlers::list_stream_messages))
.route("/api/runs/{id}/interaction/input/register", post(handlers::register_input))
.route("/api/runs/{id}/interaction/input/events", post(handlers::emit_input_event))
.route("/api/runs/{id}/interaction/input/claim/{node_id}", get(handlers::claim_input_events))
.route("/api/runs/{id}/interaction/input/ws/{node_id}", get(handlers::input_ws))

// After
.route("/api/runs/{id}/messages", post(handlers::push_message))
.route("/api/runs/{id}/messages", get(handlers::list_messages))
.route("/api/runs/{id}/messages/snapshots", get(handlers::get_snapshots))
.route("/api/runs/{id}/messages/ws", get(handlers::messages_ws))
.route("/api/runs/{id}/messages/ws/{node_id}", get(handlers::node_ws))
.route("/api/runs/{id}/interaction", get(handlers::get_interaction))
```

### Step 1.5: Delete old files

- `services/stream.rs` → deleted
- `services/input.rs` → deleted
- `handlers/interaction.rs` → deleted

### Step 1.6: Update tests

Rewrite `tests.rs` to POST to `/messages` and verify storage/retrieval.

---

## Phase 2: Node Migration

All interaction nodes change their HTTP calls. Internal logic stays the same.

### Step 2.1: dm-display

```python
# Before
requests.post(f"{server}/api/runs/{run_id}/interaction/stream", json={
    "node_id": node_id, "label": label,
    "kind": "inline", "content": content, "render": render,
})

# After
requests.post(f"{server}/api/runs/{run_id}/messages", json={
    "from": node_id,
    "tag": render,                    # "text", "image", "json", etc.
    "payload": { "label": label, "kind": "inline", "content": content },
})
```

For file artifacts:

```python
# Before
json={"node_id": node_id, "label": label, "kind": "file", "file": path, "render": render}

# After
json={"from": node_id, "tag": render, "payload": {"label": label, "kind": "file", "file": path}}
```

### Step 2.2: dm-text-input, dm-button, dm-slider, dm-input-switch

**Registration:**

```python
# Before
requests.post(f"{server}/api/runs/{run_id}/interaction/input/register", json={
    "node_id": node_id, "label": label, "widgets": widgets,
})

# After
requests.post(f"{server}/api/runs/{run_id}/messages", json={
    "from": node_id,
    "tag": "widgets",
    "payload": { "label": label, "widgets": widgets },
})
```

**WebSocket listen:**

```python
# Before
ws_url = f"{ws}/api/runs/{run_id}/interaction/input/ws/{node_id}?since={since}"

# After
ws_url = f"{ws}/api/runs/{run_id}/messages/ws/{node_id}?since={since}"
```

**Message format:**

```json
// Before
{ "type": "input.event", "event": { "output_id": "value", "value": "hello", "seq": 5 } }

// After
{ "seq": 5, "from": "web", "tag": "input", "payload": { "to": "prompt", "output_id": "value", "value": "hello" } }
```

### Step 2.3: Update dm.json for all interaction nodes

Add the `interaction` field. This is metadata for auto-panel-creation
and editor tooling:

```json
{ "id": "dm-display",    "interaction": { "emit": ["text","image","json","markdown","audio","video"] } }
{ "id": "dm-text-input", "interaction": { "emit": ["widgets"], "on": true } }
{ "id": "dm-button",     "interaction": { "emit": ["widgets"], "on": true } }
{ "id": "dm-slider",     "interaction": { "emit": ["widgets"], "on": true } }
{ "id": "dm-input-switch","interaction": { "emit": ["widgets"], "on": true } }
```

---

## Phase 3: Frontend Migration

### Step 3.1: Update `types.ts`

```typescript
// Before
export type WorkspaceWidgetType = "stream" | "input" | "terminal";

// After
export type PanelKind = "message" | "input" | "chart" | "table" | "terminal";

export interface PanelConfig {
    nodes: string[];
    tags: string[];
    [key: string]: any;
}
```

Update `WorkspaceGridItem` to use `PanelKind` and `PanelConfig`.

### Step 3.2: Update `+page.svelte` data fetching

```typescript
// Before
let interaction = $state<{ streams: any[]; inputs: any[] }>({ streams: [], inputs: [] });
async function fetchInteraction() {
    interaction = await get(`/runs/${runId}/interaction`);
}

// After
let snapshots = $state<any[]>([]);
async function fetchSnapshots() {
    snapshots = await get(`/runs/${runId}/messages/snapshots`);
}
```

**WebSocket:**

```typescript
// Before: connects to /interaction/ws
const socket = new WebSocket(`${protocol}//host/api/runs/${runId}/interaction/ws`);

// After: connects to /messages/ws
const socket = new WebSocket(`${protocol}//host/api/runs/${runId}/messages/ws`);
```

**User action emit:**

```typescript
// Before
await post(`/runs/${runId}/interaction/input/events`, {
    node_id: nodeId, output_id: outputId, value
});

// After
await post(`/runs/${runId}/messages`, {
    from: "web", tag: "input",
    payload: { to: nodeId, output_id: outputId, value }
});
```

### Step 3.3: Rename components

| Before | After |
|--------|-------|
| `DisplayStream.svelte` | `PanelMessage.svelte` |
| `DisplayMessageItem.svelte` | keep (reused by PanelMessage) |
| `InputBoard.svelte` | `PanelInput.svelte` |
| `controls/*.svelte` | keep (reused by PanelInput) |
| `WidgetTerminal.svelte` | keep unchanged |

### Step 3.4: Update PanelMessage

Change data source:

```typescript
// Before
let url = `/runs/${runId}/interaction/stream/messages?limit=50&desc=true`;
if (subscribedSourceId) url += `&source_id=${subscribedSourceId}`;

// After
let url = `/runs/${runId}/messages?limit=50&desc=true`;
const { nodes, tags } = panelConfig;
if (nodes[0] !== "*") url += `&from=${nodes.join(",")}`;
if (tags[0] !== "*") url += `&tag=${tags.join(",")}`;
```

### Step 3.5: Update PanelInput

Widget bindings now come from snapshots where `tag === "widgets"`:

```typescript
// Before
let filteredInputs = $derived(inputs.filter(...));

// After
let widgetSnapshots = $derived(
    snapshots.filter((s: any) => s.tag === "widgets")
);
// Each snapshot: { node_id, tag: "widgets", payload: { label, widgets } }
```

### Step 3.6: Update Workspace.svelte routing

```svelte
{#if dataItem.widgetType === "message"}
    <PanelMessage ... />
{:else if dataItem.widgetType === "input"}
    <PanelInput ... />
{:else if dataItem.widgetType === "terminal"}
    <WidgetTerminal ... />
{:else}
    <div class="p-4 text-muted-foreground">Unsupported: {dataItem.widgetType}</div>
{/if}
```

### Step 3.7: Update RootWidgetWrapper.svelte

Add visual identifiers for "message" kind (replace "stream" references).

### Step 3.8: Update default layout and add-widget buttons

```typescript
// Default layout
{ widgetType: "message", config: { nodes: ["*"], tags: ["*"] } }
{ widgetType: "input",   config: { nodes: ["*"], tags: ["*"] } }

// +page.svelte buttons
<Button onclick={() => addWidget("message")}>⊕ Message</Button>
<Button onclick={() => addWidget("input")}>⊕ Input</Button>
<Button onclick={() => addWidget("terminal")}>⊕ Terminal</Button>
```

---

## Verification

### Backend
- `cargo test` passes
- Start interaction-demo.yml → messages flow through new endpoints
- Swagger UI documents new endpoints

### Nodes
- dm-display messages appear in PanelMessage
- dm-text-input / dm-button / dm-slider widgets register and render
- User actions reach nodes via WebSocket
- Nodes correctly process actions and send_output

### Frontend
- Default layout loads (message + input panels)
- PanelMessage: scrollable history, source filtering, tag filtering
- PanelInput: widgets render, user can interact, actions dispatched
- Completed run: re-opening shows persisted messages and widget state
- Page refresh during live run: state recovers from snapshots
- Terminal panel: unchanged behavior

---

## Out of Scope (Future)

- `PanelChart` component (new panel kind, separate task)
- `PanelTable` component (new panel kind, separate task)
- Auto-panel creation from dm.json `interaction` declarations
- Arrow Recording / Replay integration
- Terminal panel refactoring
