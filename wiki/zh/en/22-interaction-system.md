The interaction system is the layer in Dora Manager that connects humans to data flows. Its core architectural decision is to have interactive nodes communicate with dm-server directly through the **dm Python SDK** (`dm.Message()`), replacing the old hidden Bridge node pattern. Interactive nodes simply use the HTTP API to send display messages and receive user input, requiring no network stack code -- the SDK encapsulates all HTTP communication details. This document starts from the **SDK dual-port model**, providing a deep analysis of how display nodes use `msg.send()`, how input nodes use background thread polling with `msg.get()`, and how the SDK auto-discovers runtime context.

Sources: [\_message.py](https://github.com/l1veIn/dora-manager/blob/main/sdk/python/dm/_message.py#L1-L167), [dm-display/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm_display/main.py#L1-L172), [dm-slider/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm_slider/main.py#L1-L90)

## SDK Dual-Port Model: Display and Input

The architectural foundation of the interaction system is a clear **dual-port model**. Each SDK-using interactive node has two communication modes:

- **Display (outbound) port**: Sends content to dm-server via `msg.send(tag, payload)`. The frontend retrieves the latest value through the `message_snapshots` API and renders it.
- **Input (inbound) port**: Polls for user input via a background thread using `msg.get(tag="input", after_seq=LAST_SEQ)`, filters for messages targeting itself, and sends them through standard dora output ports.

These two ports are declared through the `capabilities` field in the node's `dm.json`. The `display` family declares display capability, and the `widget_input` family declares input capability. SDK nodes do not need to declare additional input or output ports in the dora YAML -- all interactive communication is independent of the dora data plane.

```
graph TB
    subgraph "Data Plane (dora)"
        Compute["Compute Node"]
        Display["dm-display"]
        Input["dm-slider / dm-button / dm-text-input / dm-input-switch"]
    end

    subgraph "DM Plane (HTTP)"
        Server["dm-server"]
        Web["Web Browser"]
    end

    Compute -- "Arrow (path/data)" --> Display
    Display -- "HTTP POST /api/runs/.../messages" --> Server
    Input -- "HTTP GET /api/runs/.../messages (polling)" --> Server
    Server -- "WebSocket notify" --> Web
    Web -- "HTTP POST" --> Server
    Input -- "Arrow (value/click)" --> Compute
```

**Interactive nodes do not expose network ports directly**. dm-display calls the HTTP API through the SDK's `msg.send()`, and dm-slider polls for user input through the SDK's `msg.get()`. All HTTP communication with dm-server is encapsulated by the SDK in the `dm.Message` class.

Sources: [dm-display/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm.json#L41-L75), [dm-slider/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm.json#L37-L56)

## dm-display: How Display Nodes Work

**dm-display** is the display-side node of the interaction family, responsible for forwarding content from the data-plane that needs human viewing to the DM-plane. It has two standard dora input ports -- `path` (file path) and `data` (inline content) -- and then pushes content to dm-server using the SDK's `msg.send()`.

### Dual-Port Input and SDK Display

dm-display's workflow is extremely concise: it receives dora INPUT events, constructs a JSON message in `{tag, payload}` format, and sends it out through the SDK. Specifically, when the `path` port receives a file path, the payload contains `{kind: "file", file: "<relative path>"}`; when the `data` port receives inline content, the payload contains `{kind: "inline", content: "<content>"}`. The `tag` field always uses the render mode name (e.g., `"text"`, `"image"`, `"json"`).

The core send logic is done via the SDK:

```python
msg = dm.Message()
msg.send(tag, {"kind": kind, "content": content}, from_=node_id)
```

The SDK's `send` method automatically reads `DM_RUN_ID` and `DM_NODE_ID` from environment variables, constructs the correct HTTP POST request to `/api/runs/{run_id}/messages`, without requiring the node to handle URLs or authentication manually.

Sources: [dm-display/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm_display/main.py#L1-L172)

### Automatic Render Mode Inference

dm-display's `render` configuration supports an `"auto"` mode, which automatically selects the rendering method based on the input source. For `path` input, it uses file extension mapping (`.log` -> `text`, `.json` -> `json`, `.png` -> `image`, etc.); for `data` input, it infers based on the Python value type (`dict`/`list` -> `json`, others -> `text`). Developers can also force a specific mode via the `RENDER` environment variable.

Sources: [dm-display/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm_display/main.py#L15-L29)

## dm-input Family: How Input Nodes Work

Input nodes are the "human-to-data-flow" direction bridge in the interaction system. Four types are currently built in, and they follow an identical working pattern: **poll for user input messages via the SDK in a background thread, parse them, and send Arrow data through standard dora output ports**.

### Input Node Comparison Table

| Node | Capability Declared in dm.json | Output Port | Output Arrow Type | Typical Use |
|------|-------------------------------|-------------|-------------------|-------------|
| **dm-text-input** | `widget_input` | `value` | `utf8` | Text prompts, multiline input |
| **dm-button** | `widget_input` | `click` | `utf8` | Triggering actions, flow control |
| **dm-slider** | `widget_input` | `value` | `float64` | Numerical adjustment, parameter control |
| **dm-input-switch** | `widget_input` | `value` | `boolean` | Toggle switching, mode selection |

Sources: [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-text-input/dm.json#L37-L55), [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-button/dm.json#L37-L55), [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm.json#L37-L56), [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-input-switch/dm.json#L37-L56)

### Unified Message Polling Pattern

The core logic of all input nodes is completely identical -- they poll for user input messages via the SDK in a background thread, filter for messages targeting themselves, perform type conversion, and send to the semantic output port. Taking dm-slider as an example:

```python
def poll_inputs(msg, node, yaml_id):
    last_seq = 0
    while node.is_running():
        messages = msg.get(tag="input", after_seq=last_seq)
        for item in messages:
            last_seq = item["seq"]
            payload = item["payload"]
            if payload.get("to") == yaml_id:
                node.send_output("value", pa.array([float(payload["value"])]))
        time.sleep(0.1)

msg = dm.Message()
thread = threading.Thread(target=poll_inputs, args=(msg, node, yaml_id), daemon=True)
thread.start()
```

The difference between each node lies only in the `normalize_output` (type conversion) function -- dm-slider converts values to `float64`, dm-input-switch to `boolean`, and dm-text-input and dm-button keep them as `utf8` strings. The SDK's `msg.get()` method automatically polls for messages after the sequence number specified by `after_seq`, ensuring each message is processed only once.

Sources: [dm-slider/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm_slider/main.py#L1-L90), [dm-button/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-button/dm_button/main.py#L1-L81), [dm-input-switch/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-input-switch/dm_input_switch/main.py#L1-L77), [dm-text-input/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-text-input/dm_text_input/main.py#L1-L104)

## SDK Auto-Discovery of Runtime Context

The SDK's `dm.Message` class automatically discovers runtime context from environment variables during initialization:

| Environment Variable | Purpose |
|----------------------|---------|
| `DM_RUN_ID` | Unique identifier for the current run instance, used to construct HTTP API paths |
| `DM_NODE_ID` | Node identity within the dataflow, used for `msg.send()`'s `from_` parameter and `msg.get()` filtering |

This means node code never needs to manually pass or hardcode these values -- the SDK reads `os.environ` automatically on first use. If these variables are missing from the environment, `msg.send()` and `msg.get()` raise clear error messages.

Sources: [\_message.py](https://github.com/l1veIn/dora-manager/blob/main/sdk/python/dm/_message.py#L1-L167)

## Widget Registration: Via msg.send("widgets", ...)

Interactive nodes register widget descriptions via the SDK at startup, enabling the frontend to automatically render the corresponding UI controls. Registration happens before the node's main loop begins:

```python
def main():
    msg = dm.Message()
    node = Node()

    # Register widget -- sent immediately at node startup
    msg.send("widgets", {
        "label": "Temperature (°C)",
        "widgets": {
            "value": {
                "type": "slider",
                "label": "Temperature",
                "min": -20, "max": 50, "step": 1, "default": 20
            }
        }
    })

    # Start background thread for input polling
    thread = threading.Thread(target=poll_inputs, args=(msg, node, yaml_id), daemon=True)
    thread.start()

    # Main loop -- handle dora Arrow input (optional)
    for event in node:
        ...
```

SDK node widget registration is done via `msg.send("widgets", payload)`, sharing the same HTTP API endpoint as regular messages. dm-server stores it as a `tag="widgets"` snapshot, and the frontend retrieves it via `GET /api/runs/{id}/messages/snapshots` for rendering.

Sources: [dm_sdk_demo/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-sdk-demo/dm_sdk_demo/main.py#L1-L95)

## dm-server's Message Service

dm-server's `MessageService` persists messages using SQLite. Each `push` operation writes to both the `messages` history table (append-only) and updates the `message_snapshots` table (using `UPSERT` semantics to ensure each `(node_id, tag)` combination always holds the latest state). This dual-table design supports both historical replay and fast snapshot queries.

```
sequenceDiagram
    participant Node as SDK Node
    participant API as dm-server REST
    participant DB as SQLite
    participant WS as WebSocket
    participant Web as Web Browser

    Note over Node,Web: Initialization Phase
    Node->>API: POST /api/runs/{id}/messages {"tag":"widgets","payload":{...}}
    API->>DB: MessageService.push() → upsert snapshots
    API->>WS: broadcast notification

    Note over Node,Web: Display Path
    Node->>API: POST /api/runs/{id}/messages {"tag":"text","payload":{...}}
    API->>DB: INSERT messages + UPSERT snapshots
    API->>WS: broadcast notification
    Web->>DB: GET /snapshots → get latest snapshot

    Note over Node,Web: Input Path
    Web->>API: POST /api/runs/{id}/messages {"tag":"input","payload":{to:"slider",value:25}}
    API->>DB: INSERT messages + UPSERT snapshots
    API->>WS: broadcast notification
    Node->>API: GET /messages?tag=input&after_seq=N
    Node->>Node: filter payload.to == yaml_id → send_output
```

Sources: [message.rs](https://github.com/l1veIn/dora-manager/blob/main/crates/dm-server/src/services/message.rs#L1-L243)

## Capability Binding Declaration Specification

Interactive node capabilities are declared through the `capabilities` field in `dm.json`. SDK nodes are recognized by the system through their metadata. Two capability families are currently supported:

### `display` Family

Declares that the node is a display node, bridging data-plane content to the DM-plane. Each binding declares a data channel:

| Field | Meaning | Typical Value |
|-------|---------|---------------|
| `role` | The node's role within this family | `"source"` |
| `port` | dora port name, identifying the junction between data-plane and DM-plane | `"data"` or `"path"` |
| `channel` | Semantic channel on the DM side | `"inline"` or `"artifact"` |
| `media` | List of supported render types | `["text", "json", "markdown"]` |
| `lifecycle` | Lifecycle hints | `[]` |

Sources: [dm-display/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm.json#L41-L75), [dm-capability-binding-v0.md](https://github.com/l1veIn/dora-manager/blob/main/docs/design/dm-capability-binding-v0.md#L56-L100)

### `widget_input` Family

Declares that the node is an input node, receiving DM-plane user actions and injecting them into the data-plane. Each binding declares an interaction channel:

| Field | Meaning | Typical Value |
|-------|---------|---------------|
| `role` | Node role | `"widget"` |
| `channel` | Channel semantics: `"register"` for registration, `"input"` for data reception | `"register"` / `"input"` |
| `port` | dora output port name (only present when `channel=input`) | `"value"` / `"click"` |
| `media` | Interaction data type | `["text"]`, `["number"]`, `["boolean"]`, `["pulse"]` |
| `lifecycle` | Lifecycle constraints | `["run_scoped", "stop_aware"]` |

Each widget_input node typically declares two bindings: one `channel: "register"` describing widget registration information, and one `channel: "input"` describing the data reception port. SDK nodes use these two bindings for widget registration (`msg.send("widgets", ...)` at startup) and input polling (background thread with `msg.get(tag="input", ...)`) respectively.

Sources: [dm-slider/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm.json#L37-L56), [dm-capability-binding-v0.md](https://github.com/l1veIn/dora-manager/blob/main/docs/design/dm-capability-binding-v0.md#L102-L148)

## Extending Custom Interaction Nodes

Creating new interaction nodes follows a strict contract and requires no modifications to dm-core code:

**Creating a new display node**:
1. Declare a `display` capability in `dm.json`, specifying `port` and `channel`
2. In the node implementation, initialize `dm.Message()` and push content via `msg.send(tag, payload)`
3. The SDK automatically handles HTTP communication and run ID discovery

**Creating a new input node**:
1. Declare a `widget_input` capability in `dm.json`, specifying bindings for both `channel: "register"` and `channel: "input"`
2. In the node implementation, start a background thread that polls for input via `msg.get(tag="input", after_seq=...)`
3. Filter `payload.to == yaml_id`, perform type conversion, and send through the declared output port

**Widget type registration**: Send widget descriptions using `msg.send("widgets", payload)` in the node code. The SDK node widget type mapping table is defined by the frontend component library, currently supporting input, textarea, button, select, slider, switch, radio, checkbox, path, file, and other types.

Sources: [dm-sdk-demo/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-sdk-demo/dm_sdk_demo/main.py#L1-L95), [InteractionPane.svelte](https://github.com/l1veIn/dora-manager/blob/main/web/src/routes/runs/[id]/InteractionPane.svelte#L200-L321)

## Related Reading

- [Capability Binding: Node Capability Declaration and Runtime Role Binding](23-capability-binding.md) -- a deeper look at the complete design of the capability system
- [Built-in Nodes Overview: From Media Collection to AI Inference](07-builtin-nodes.md) -- understanding the position of interaction nodes in the complete node ecosystem
- [Reactive Widgets: Widget Registry, Dynamic Rendering, and WebSocket Parameter Injection](20-reactive-widgets.md) -- detailed documentation of the frontend widget rendering mechanism
