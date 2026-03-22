# P1: Read-Only Graph Canvas

Render the dataflow YAML as an interactive node graph. Users can view, pan, zoom, and drag to reposition nodes. No topology editing (no add/delete/connect). Positions are persisted in `view.json`.

## Prerequisites

- `@xyflow/svelte` and `@dagrejs/dagre` installed in `web/`
- Familiarity with the YAML format in `tests/dataflows/qwen-dev.yml`

```bash
cd web && npm install @xyflow/svelte @dagrejs/dagre
```

## Decisions

| Item | Decision |
|------|----------|
| Save timing | **Explicit** — Save button writes both YAML and view.json |
| Auto layout | Dagre LR when view.json is missing or incomplete |
| Virtual nodes | `dora/timer/*` and `panel/*` sources rendered as lightweight virtual nodes |

---

## Backend Changes

All backend changes follow the `config.json` pattern (see `repo::read_config` / `repo::write_config`).

### 1. `crates/dm-core/src/dataflow/paths.rs`

Add constant and helper:

```rust
pub const FLOW_VIEW_FILE: &str = "view.json";

pub fn flow_view_path(dir: &Path) -> PathBuf {
    dir.join(FLOW_VIEW_FILE)
}
```

### 2. `crates/dm-core/src/dataflow/repo.rs`

Add two functions mirroring `read_config`/`write_config`:

```rust
pub fn read_view(home: &Path, name: &str) -> Result<serde_json::Value> {
    let path = flow_view_path(&dataflow_dir(home, name));
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read view for '{}'", name))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse view for '{}'", name))
}

pub fn write_view(home: &Path, name: &str, view: &serde_json::Value) -> Result<()> {
    let dir = dataflow_dir(home, name);
    fs::create_dir_all(&dir)?;
    let path = flow_view_path(&dir);
    fs::write(
        &path,
        serde_json::to_string_pretty(view).context("Failed to serialize view.json")?,
    )
    .with_context(|| format!("Failed to write {}", path.display()))
}
```

Import `flow_view_path` at top of file.

### 3. `crates/dm-core/src/dataflow/model.rs`

Add optional `view` field to `DataflowProject`:

```rust
pub struct DataflowProject {
    pub name: String,
    pub yaml: String,
    pub meta: FlowMeta,
    pub executable: DataflowExecutableSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<serde_json::Value>,        // NEW
}
```

### 4. `crates/dm-core/src/dataflow/service.rs`

- In `get()`, read view and populate the field:
  ```rust
  let view = repo::read_view(home, name).ok();
  Ok(DataflowProject { name: ..., yaml, meta, executable, view })
  ```
- Add two public functions:
  ```rust
  pub fn get_flow_view(home: &Path, name: &str) -> Result<serde_json::Value> {
      repo::read_view(home, name)
  }
  pub fn save_flow_view(home: &Path, name: &str, view: &serde_json::Value) -> Result<()> {
      repo::write_view(home, name, view)
  }
  ```

### 5. `crates/dm-core/src/dataflow/mod.rs`

Re-export the new functions:

```rust
pub use service::{
    ..., get_flow_view, save_flow_view,
};
```

### 6. `crates/dm-server/src/handlers/dataflow.rs`

Add two handlers:

```rust
/// GET /api/dataflows/:name/view
pub async fn get_dataflow_view(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::get_flow_view(&state.home, &name) {
        Ok(view) => Json(view).into_response(),
        Err(e) => dataflow_not_found_or_err(e, &name).into_response(),
    }
}

/// POST /api/dataflows/:name/view
pub async fn save_dataflow_view(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(view): Json<serde_json::Value>,
) -> impl IntoResponse {
    match dm_core::dataflow::save_flow_view(&state.home, &name, &view) {
        Ok(()) => Json(serde_json::json!({ "message": "View saved" })).into_response(),
        Err(e) => err(e).into_response(),
    }
}
```

### 7. `crates/dm-server/src/handlers/mod.rs`

Add to the re-export list:

```rust
pub use dataflow::{..., get_dataflow_view, save_dataflow_view};
```

### 8. `crates/dm-server/src/main.rs`

Add routes after the existing dataflow history routes:

```rust
.route("/api/dataflows/{name}/view", get(handlers::get_dataflow_view))
.route("/api/dataflows/{name}/view", post(handlers::save_dataflow_view))
```

---

## Frontend Changes

### File Structure

```
web/src/routes/dataflows/[id]/components/
├── GraphEditorTab.svelte          # NEW — main tab component
└── graph/
    ├── types.ts                   # NEW — type definitions
    ├── yaml-graph.ts              # NEW — YAML ↔ Graph conversion
    ├── auto-layout.ts             # NEW — dagre layout
    └── DmNode.svelte              # NEW — custom node component
```

### 1. `graph/types.ts`

```typescript
import type { Node, Edge } from '@xyflow/svelte';

// Data payload attached to each SvelteFlow node
export interface DmNodeData {
  label: string;          // display label (yaml id)
  nodeType: string;       // node type id (from YAML `node:` field)
  inputs: string[];       // input port ids
  outputs: string[];      // output port ids
  isVirtual?: boolean;    // true for dora/timer and panel pseudo-nodes
  virtualKind?: 'timer' | 'panel';
}

export interface ViewNodePosition {
  x: number;
  y: number;
}

export interface ViewJson {
  viewport?: { x: number; y: number; zoom: number };
  nodes?: Record<string, ViewNodePosition>;
}

export type DmFlowNode = Node<DmNodeData>;
export type DmFlowEdge = Edge;

// Classify an input source string
export type InputSource =
  | { type: 'node'; sourceId: string; outputPort: string }
  | { type: 'dora'; raw: string }      // dora/timer/millis/2000
  | { type: 'panel'; widgetId: string } // panel/device_id

export function classifyInput(value: string): InputSource {
  if (value.startsWith('dora/')) return { type: 'dora', raw: value };
  if (value.startsWith('panel/')) return { type: 'panel', widgetId: value.split('/')[1] };
  const slashIdx = value.indexOf('/');
  if (slashIdx > 0) {
    return { type: 'node', sourceId: value.substring(0, slashIdx), outputPort: value.substring(slashIdx + 1) };
  }
  return { type: 'dora', raw: value }; // fallback
}
```

### 2. `graph/yaml-graph.ts`

Core conversion logic:

```typescript
import YAML from 'yaml';                         // or js-yaml
import type { DmFlowNode, DmFlowEdge, DmNodeData, ViewJson } from './types';
import { classifyInput } from './types';
import { applyDagreLayout } from './auto-layout';

interface YamlNode {
  id: string;
  node?: string;
  path?: string;
  inputs?: Record<string, string>;
  outputs?: string[];
  config?: Record<string, unknown>;
  env?: Record<string, unknown>;
  widgets?: Record<string, unknown>;
}

export function yamlToGraph(
  yamlStr: string,
  viewJson: ViewJson = {}
): { nodes: DmFlowNode[]; edges: DmFlowEdge[] } {
  let parsed: { nodes?: YamlNode[] };
  try {
    parsed = YAML.parse(yamlStr) || {};
  } catch {
    return { nodes: [], edges: [] };
  }

  const yamlNodes = parsed.nodes || [];
  const nodes: DmFlowNode[] = [];
  const edges: DmFlowEdge[] = [];
  const virtualNodeIds = new Set<string>();

  // First pass: create real nodes
  for (const yn of yamlNodes) {
    const pos = viewJson.nodes?.[yn.id];
    const inputKeys = yn.inputs ? Object.keys(yn.inputs) : [];
    const outputKeys = yn.outputs || [];

    nodes.push({
      id: yn.id,
      type: 'dmNode',
      position: pos ? { x: pos.x, y: pos.y } : { x: 0, y: 0 },
      data: {
        label: yn.id,
        nodeType: yn.node || yn.path || 'unknown',
        inputs: inputKeys,
        outputs: outputKeys,
      },
    });
  }

  // Second pass: derive edges + virtual nodes
  for (const yn of yamlNodes) {
    if (!yn.inputs) continue;
    for (const [inputPort, sourceStr] of Object.entries(yn.inputs)) {
      const src = classifyInput(sourceStr);

      if (src.type === 'node') {
        edges.push({
          id: `e-${src.sourceId}-${src.outputPort}-${yn.id}-${inputPort}`,
          source: src.sourceId,
          target: yn.id,
          sourceHandle: `out-${src.outputPort}`,
          targetHandle: `in-${inputPort}`,
        });
      } else if (src.type === 'dora') {
        // Create virtual timer node if not exists
        const virtualId = `__virtual_${src.raw.replace(/\//g, '_')}`;
        if (!virtualNodeIds.has(virtualId)) {
          virtualNodeIds.add(virtualId);
          const pos = viewJson.nodes?.[virtualId];
          // Extract a friendly label from the dora source
          const parts = src.raw.split('/');
          const label = parts.length >= 4
            ? `Timer ${parts[3]}ms`
            : src.raw;
          nodes.push({
            id: virtualId,
            type: 'dmNode',
            position: pos ? { x: pos.x, y: pos.y } : { x: 0, y: 0 },
            data: {
              label,
              nodeType: src.raw,
              inputs: [],
              outputs: ['tick'],
              isVirtual: true,
              virtualKind: 'timer',
            },
          });
        }
        edges.push({
          id: `e-${virtualId}-tick-${yn.id}-${inputPort}`,
          source: virtualId,
          target: yn.id,
          sourceHandle: 'out-tick',
          targetHandle: `in-${inputPort}`,
        });
      } else if (src.type === 'panel') {
        // Panel virtual node — one shared node for all panel/ outputs
        const virtualId = '__virtual_panel';
        if (!virtualNodeIds.has(virtualId)) {
          virtualNodeIds.add(virtualId);
          const pos = viewJson.nodes?.[virtualId];
          nodes.push({
            id: virtualId,
            type: 'dmNode',
            position: pos ? { x: pos.x, y: pos.y } : { x: 0, y: 0 },
            data: {
              label: 'Panel Inputs',
              nodeType: 'panel',
              inputs: [],
              outputs: [],  // outputs will be collected dynamically
              isVirtual: true,
              virtualKind: 'panel',
            },
          });
        }
        // Add the widget output to the virtual panel node's port list
        const panelNode = nodes.find(n => n.id === virtualId);
        if (panelNode && !panelNode.data.outputs.includes(src.widgetId)) {
          panelNode.data.outputs.push(src.widgetId);
        }
        edges.push({
          id: `e-panel-${src.widgetId}-${yn.id}-${inputPort}`,
          source: virtualId,
          target: yn.id,
          sourceHandle: `out-${src.widgetId}`,
          targetHandle: `in-${inputPort}`,
        });
      }
    }
  }

  // Apply auto-layout if any node lacks a position
  const needsLayout = nodes.some(n => n.position.x === 0 && n.position.y === 0
    && !viewJson.nodes?.[n.id]);
  if (needsLayout) {
    return applyDagreLayout(nodes, edges);
  }

  return { nodes, edges };
}

// Build view.json from current canvas state
export function buildViewJson(
  nodes: DmFlowNode[],
  viewport?: { x: number; y: number; zoom: number }
): ViewJson {
  const view: ViewJson = {};
  if (viewport) view.viewport = viewport;
  view.nodes = {};
  for (const n of nodes) {
    view.nodes[n.id] = { x: n.position.x, y: n.position.y };
  }
  return view;
}
```

> **Note**: Install a YAML parser for frontend. The `yaml` npm package (eemeli/yaml) is recommended:
> ```bash
> npm install yaml
> ```

### 3. `graph/auto-layout.ts`

```typescript
import dagre from '@dagrejs/dagre';
import type { DmFlowNode, DmFlowEdge } from './types';

const NODE_WIDTH = 260;
const NODE_HEIGHT = 120;

export function applyDagreLayout(
  nodes: DmFlowNode[],
  edges: DmFlowEdge[]
): { nodes: DmFlowNode[]; edges: DmFlowEdge[] } {
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: 'LR', nodesep: 60, ranksep: 120 });

  for (const node of nodes) {
    g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }
  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  dagre.layout(g);

  const layoutNodes = nodes.map(node => {
    const pos = g.node(node.id);
    return {
      ...node,
      position: {
        x: pos.x - NODE_WIDTH / 2,
        y: pos.y - NODE_HEIGHT / 2,
      },
    };
  });

  return { nodes: layoutNodes, edges };
}
```

### 4. `graph/DmNode.svelte`

Custom node component with input/output handles:

```svelte
<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { DmNodeData } from './types';

  let { data, id } = $props<{ data: DmNodeData; id: string }>();
</script>

<div
  class="dm-node"
  class:dm-node--virtual={data.isVirtual}
  class:dm-node--timer={data.virtualKind === 'timer'}
  class:dm-node--panel={data.virtualKind === 'panel'}
>
  <div class="dm-node__header">
    <span class="dm-node__label">{data.label}</span>
    {#if !data.isVirtual}
      <span class="dm-node__type">{data.nodeType}</span>
    {/if}
  </div>

  <div class="dm-node__ports">
    <div class="dm-node__inputs">
      {#each data.inputs as port, i}
        <div class="dm-node__port">
          <Handle
            type="target"
            position={Position.Left}
            id={`in-${port}`}
            style="top: {32 + 24 + i * 22}px;"
          />
          <span class="dm-node__port-label">{port}</span>
        </div>
      {/each}
    </div>

    <div class="dm-node__outputs">
      {#each data.outputs as port, i}
        <div class="dm-node__port dm-node__port--output">
          <span class="dm-node__port-label">{port}</span>
          <Handle
            type="source"
            position={Position.Right}
            id={`out-${port}`}
            style="top: {32 + 24 + i * 22}px;"
          />
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .dm-node {
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    min-width: 200px;
    font-size: 13px;
    box-shadow: 0 1px 3px rgba(0,0,0,.08);
  }
  .dm-node--virtual {
    border-style: dashed;
    opacity: 0.85;
  }
  .dm-node--timer { border-color: hsl(200 70% 55%); }
  .dm-node--panel { border-color: hsl(280 60% 55%); }

  .dm-node__header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid hsl(var(--border));
    background: hsl(var(--muted));
    border-radius: 8px 8px 0 0;
  }
  .dm-node__label {
    font-weight: 600;
    font-family: var(--font-mono, monospace);
  }
  .dm-node__type {
    font-size: 11px;
    color: hsl(var(--muted-foreground));
    margin-left: auto;
  }

  .dm-node__ports {
    display: flex;
    justify-content: space-between;
    padding: 8px 12px;
    gap: 16px;
  }
  .dm-node__port {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    position: relative;
  }
  .dm-node__port--output {
    justify-content: flex-end;
  }
  .dm-node__port-label {
    font-size: 11px;
    color: hsl(var(--foreground) / 0.7);
    font-family: var(--font-mono, monospace);
  }
</style>
```

### 5. `GraphEditorTab.svelte`

Main tab component wrapping SvelteFlow:

```svelte
<script lang="ts">
  import { SvelteFlow, Controls, MiniMap, Background } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import { post } from '$lib/api';
  import { toast } from 'svelte-sonner';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Save } from 'lucide-svelte';
  import DmNode from './graph/DmNode.svelte';
  import { yamlToGraph, buildViewJson } from './graph/yaml-graph';
  import type { ViewJson, DmFlowNode, DmFlowEdge } from './graph/types';

  let {
    dataflowName,
    yamlStr = '',
    viewJson = {} as ViewJson,
  } = $props<{
    dataflowName: string;
    yamlStr?: string;
    viewJson?: ViewJson;
  }>();

  const nodeTypes = { dmNode: DmNode };

  let { nodes: initialNodes, edges: initialEdges } = yamlToGraph(yamlStr, viewJson);
  let nodes = $state<DmFlowNode[]>(initialNodes);
  let edges = $state<DmFlowEdge[]>(initialEdges);
  let isSaving = $state(false);

  // Refresh graph when yamlStr changes (e.g. switching from YAML tab)
  $effect(() => {
    const result = yamlToGraph(yamlStr, viewJson);
    nodes = result.nodes;
    edges = result.edges;
  });

  async function saveView() {
    isSaving = true;
    try {
      const view = buildViewJson(nodes);
      await post(`/dataflows/${dataflowName}/view`, view);
      toast.success('View saved');
    } catch (e: any) {
      toast.error(`Save failed: ${e.message}`);
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="h-full flex flex-col min-h-0 w-full">
  <div class="flex justify-end mb-3">
    <Button variant="outline" size="sm" disabled={isSaving} onclick={saveView}>
      <Save class="mr-2 size-4" />
      {isSaving ? 'Saving...' : 'Save layout'}
    </Button>
  </div>

  <div class="flex-1 min-h-0 relative border rounded-md shadow-sm overflow-hidden">
    <SvelteFlow {nodes} {edges} {nodeTypes} fitView>
      <Controls />
      <MiniMap />
      <Background />
    </SvelteFlow>
  </div>
</div>
```

### 6. `+page.svelte` Modifications

1. Import `GraphEditorTab`
2. Add a `"graph"` tab as the **first tab** (set `value="graph"` as default)
3. Pass `dataflow.yaml` and `dataflow.view` to `GraphEditorTab`
4. When `activeTab` changes TO `"graph"`, pass the latest `dataflow.yaml` so graph re-parses

```svelte
<!-- Add to Tabs.List -->
<Tabs.Trigger value="graph" class="gap-2">
  <GitBranch class="size-4" />
  Graph Editor
</Tabs.Trigger>

<!-- Add Tab Content -->
<Tabs.Content value="graph" class="flex-1 flex flex-col min-h-0 overflow-hidden mt-0">
  {#if activeTab === 'graph' && dataflow?.yaml !== undefined}
    <GraphEditorTab
      {dataflowName}
      yamlStr={dataflow.yaml || ''}
      viewJson={dataflow.view || {}}
    />
  {/if}
</Tabs.Content>
```

Change default tab from `"yaml"` to `"graph"`:
```svelte
<Tabs.Root value="graph" ...>
```

---

## Verification

### Cargo test

```bash
cargo test -p dm-core -- tests_dataflow
```

Existing tests + new view read/write tests should pass.

### Svelte check

```bash
cd web && npm run check
```

### Manual browser test

1. Open a dataflow detail page
2. "Graph Editor" tab is the default and renders nodes + edges
3. Nodes match the YAML (count, labels, connections)
4. Virtual nodes appear for `dora/timer/*` and `panel/*` inputs
5. Drag nodes → positions change
6. Click "Save layout" → `view.json` persisted
7. Refresh page → positions restored
8. Switch to YAML tab and back → graph re-renders correctly
