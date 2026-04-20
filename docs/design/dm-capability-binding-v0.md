# DM Capability Binding v0

> Status: draft for implementation pilot  
> Depends on: [panel-ontology-memo.md](/Users/yangchen/Desktop/dora-manager/docs/design/panel-ontology-memo.md)

## 1. Purpose

This document formalizes the first explicit contract for the DM plane.

The goal is not to redesign transport in one step. The goal is to make the
existing "vertical binding" truth visible and structured so the project can
stop treating DM interaction as ad hoc node-local glue.

## 2. Core Judgment

`dora` ports remain the data plane.

DM capability bindings are a separate, explicit plane that attaches selected
nodes or ports to DM runtime behavior such as:

- widget registration
- browser-to-node input
- run-scoped display emission
- future lifecycle/control participation

This binding is real, but it is not a normal graph edge. It is a vertical
binding from the canvas into the DM plane.

## 3. Non-Goals For v0

This round does not:

- replace `dora` ports
- redesign run-scoped messaging transport
- remove all legacy node-local HTTP/WS code
- add editor visualization for capability bindings
- define a full multi-language SDK

## 4. Schema Shape

`dm.json` expresses fine-grained bindings inside structured `capabilities` entries:

```json
{
  "capabilities": [
    "configurable",
    {
      "name": "display",
      "bindings": [
        {
          "role": "source",
          "port": "data",
          "channel": "inline",
          "media": ["text", "json", "markdown"]
        }
      ]
    }
  ]
}
```

### 4.1 Top-Level Fields

- `capabilities`
  - Mixed list of coarse string tags and structured capability objects.

### 4.2 Binding Fields

- `name`
  - Capability family name such as `display` or `widget_input`.
- `family`
  - Legacy compatibility field only. In the converged schema, the family lives on
    the containing capability object.
- `role`
  - The node's role inside that family.
- `port`
  - Optional `dora` port where the data plane meets the DM plane.
- `channel`
  - The DM-side semantic channel.
- `media`
  - Optional payload/render hints used by DM surfaces.
- `lifecycle`
  - Optional lifecycle hints such as `run_scoped` or `stop_aware`.
- `description`
  - Optional human-readable explanation for tooling surfaces.

## 5. Family Model In v0

Keep v0 intentionally small.

### 5.1 `widget_input`

For nodes that participate in browser input.

Typical bindings:

- `channel = "register"`
  - Node publishes widget definition to the DM plane.
- `channel = "input"`
  - DM plane delivers user input back to the node's data-plane port.

### 5.2 `display`

For nodes that expose human-visible output through the DM plane.

Typical bindings:

- `channel = "inline"`
  - Inline content such as text, json, markdown.
- `channel = "artifact"`
  - File/artifact-backed output such as image, audio, video.

### 5.3 `run_control`

Reserved for lifecycle participation such as stop awareness. This family is not
required for the first implementation pilot, but the schema leaves room for it.

## 6. Node-Level vs Port-Level Meaning

Bindings are binding-centric, not port-centric.

- A binding may point to a `port` when the DM plane meets the `dora` plane.
- Some DM-plane semantics may be node-level and therefore omit `port`.

This matters because not every DM concern should be forced into fake data ports.

## 7. Pilot Scope

The first pilot covers two builtin nodes only:

1. `dm-text-input`
2. `dm-display`

### 7.1 `dm-text-input`

```json
{
  "capabilities": [
    "configurable",
    {
      "name": "widget_input",
      "bindings": [
        {
          "role": "widget",
          "channel": "register",
          "media": ["widgets"],
          "lifecycle": ["run_scoped", "stop_aware"],
          "description": "Registers a text widget with the DM interaction plane."
        },
        {
          "role": "widget",
          "port": "value",
          "channel": "input",
          "media": ["text"],
          "lifecycle": ["run_scoped", "stop_aware"],
          "description": "Emits submitted user input onto the value output port."
        }
      ]
    }
  ]
}
```

### 7.2 `dm-display`

```json
{
  "capabilities": [
    "configurable",
    {
      "name": "display",
      "bindings": [
        {
          "role": "source",
          "port": "data",
          "channel": "inline",
          "media": ["text", "json", "markdown"],
          "description": "Relays inline display content into the DM interaction plane."
        },
        {
          "role": "source",
          "port": "path",
          "channel": "artifact",
          "media": ["image", "audio", "video", "text", "json", "markdown"],
          "description": "Relays artifact-backed display content into the DM interaction plane."
        }
      ]
    }
  ]
}
```

## 8. Runtime Interpretation In v0

This round now includes a first narrow lowering path, not only read-only metadata:

1. parse and return structured capabilities, including binding-bearing capability objects
2. preserve them through existing node-loading and node-status APIs
3. expose them in node-facing surfaces such as the detail page
4. during transpile, auto-inject one hidden `dm-bridge` system node for runs that declare supported bindings
5. auto-inject the hidden edges and env needed for builtin widget/display nodes to talk to that bridge through the dora data plane

This is intentionally narrow:

- widget registration and browser input can be lowered through the hidden bridge path
- display emission can be lowered through the hidden bridge path
- the bridge is injected by transpile rather than authored into user YAML
- the author-visible graph still does not render the bridge as a normal business node

## 9. Migration Rule

Existing ad hoc `interaction` metadata is legacy.

For this pilot:

- builtin nodes in the repo should migrate to structured `capabilities`
- older third-party nodes may continue to carry legacy `interaction`
- older third-party nodes may also carry legacy top-level `dm`; loaders should
  ingest it and normalize it into `capabilities`

## 10. Why This Is The Right Next Step

This round turns a hidden architectural truth into an explicit contract:

- `ports` stay honest as data-plane edges
- the DM plane stops being a pile of per-node magic strings
- node surfaces can start showing the real binding model
- future runtime work has a schema to target

It is small enough to ship now and strong enough to guide the next rounds.
