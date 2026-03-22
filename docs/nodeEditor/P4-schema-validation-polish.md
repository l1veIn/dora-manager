# P4: Enhanced — Schema Validation, Polish & UX

Visual connection validation using Port Schema, UI polish (animations, edge styles, theme), and MiniMap/Controls refinements.

## Prerequisites

- **P1 + P2 + P3 completed**
- Port Schema definitions exist in `dm.json` (see `docs/design/dm-port-schema.md`)
- Backend transpile already has `validate_port_schemas()` in `passes.rs`

## Scope

| Feature | Description |
|---------|-------------|
| Connection validation | Color-coded edges based on Port Schema compatibility |
| Schema tooltip | Hover over port handle to see schema type info |
| Edge styling | Animated, curved, color-coded edges |
| Node status indicators | Visual cues for unresolved/missing nodes |
| Theme integration | Dark/light mode aware custom styles |
| Fit-to-view controls | Auto-fit, zoom-to-selection |
| Copy/paste nodes | Duplicate a node with its config |
| Undo/Redo | Basic undo stack for graph operations |

---

## Schema Validation Architecture

### Existing Backend Logic

The transpiler's `validate_port_schemas()` in `crates/dm-core/src/dataflow/transpile/passes.rs` already implements:

1. Load `dm.json` for both source and target nodes
2. Find port declarations by `port.id`
3. Skip if either port lacks `schema` (gradual validation)
4. Skip if node has `dynamic_ports: true` and port isn't declared
5. Resolve `$ref` references
6. Call `check_compatibility(out_schema, in_schema)` → pass/fail

The schema compatibility checker in `crates/dm-core/src/node/schema/` supports:
- Exact type match → ✅
- Widening (int32→int64, float32→float64, utf8→largeutf8) → ✅ 
- Fixed-size list → variable list → ✅
- Cross-domain mismatch → ❌

### Frontend: New Validate API

#### Backend — New endpoint

```
GET /api/dataflows/{name}/validate
```

Returns schema validation diagnostics for the current YAML:

```jsonc
{
  "diagnostics": [
    {
      "yaml_id": "dora-vad",
      "node_id": "dora-vad",
      "kind": {
        "IncompatiblePortSchema": {
          "output_port": "dm-microphone/audio",
          "input_port": "audio",
          "reason": "Type mismatch: expected struct, got utf8"
        }
      }
    }
  ]
}
```

#### Backend implementation

In `crates/dm-core/src/dataflow/service.rs`:

```rust
pub fn validate(home: &Path, name: &str) -> Result<Vec<TranspileDiagnostic>> {
    let yaml = repo::read_yaml(home, name)?;
    let graph = transpile::passes::parse(&yaml)?;
    let ctx = TranspileContext { home, ... };
    let mut diags = Vec::new();
    transpile::passes::validate_port_schemas(&ctx, &graph, &mut diags);
    Ok(diags)
}
```

Expose via handler + route as done for other endpoints.

---

## Frontend Changes

### 1. Edge Validation Coloring

After loading graph, call the validate API and color edges:

```typescript
// Types
type EdgeValidation = 'valid' | 'warning' | 'error' | 'unvalidated';

interface ValidatedEdge extends DmFlowEdge {
  data?: { validation: EdgeValidation; reason?: string };
}

// After yamlToGraph, call validate
async function validateEdges(dataflowName: string, edges: DmFlowEdge[]): Promise<ValidatedEdge[]> {
  try {
    const result: any = await get(`/dataflows/${dataflowName}/validate`);
    const diagnostics = result.diagnostics || [];

    // Build a lookup: "sourceId/outputPort → inputNodeId/inputPort" → diagnostic
    const diagMap = new Map<string, any>();
    for (const d of diagnostics) {
      if (d.kind.IncompatiblePortSchema) {
        const key = `${d.kind.IncompatiblePortSchema.output_port}→${d.yaml_id}/${d.kind.IncompatiblePortSchema.input_port}`;
        diagMap.set(key, d);
      }
    }

    return edges.map(edge => {
      const sourcePort = edge.sourceHandle?.replace('out-', '') || '';
      const targetPort = edge.targetHandle?.replace('in-', '') || '';
      const key = `${edge.source}/${sourcePort}→${edge.target}/${targetPort}`;

      const diag = diagMap.get(key);
      if (diag) {
        return { ...edge, data: { validation: 'error', reason: diag.kind.IncompatiblePortSchema.reason } };
      }
      return { ...edge, data: { validation: 'valid' } };
    });
  } catch {
    // If validation fails, mark all edges as unvalidated
    return edges.map(e => ({ ...e, data: { validation: 'unvalidated' } }));
  }
}
```

### 2. Custom Edge Component `DmEdge.svelte`

```svelte
<script lang="ts">
  import { BaseEdge, getBezierPath } from '@xyflow/svelte';

  let { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data } = $props();

  let [path, labelX, labelY] = $derived(
    getBezierPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition })
  );

  let strokeColor = $derived(() => {
    switch (data?.validation) {
      case 'valid': return 'hsl(142 70% 45%)';      // green
      case 'warning': return 'hsl(45 93% 47%)';     // amber
      case 'error': return 'hsl(0 84% 60%)';        // red
      default: return 'hsl(var(--muted-foreground))'; // grey
    }
  });
</script>

<BaseEdge {id} {path} style="stroke: {strokeColor()}; stroke-width: 2;" />

{#if data?.validation === 'error' && data?.reason}
  <foreignObject x={labelX - 75} y={labelY - 12} width="150" height="24">
    <div class="text-[10px] text-red-500 bg-card border border-red-300 rounded px-1.5 py-0.5
                text-center truncate" title={data.reason}>
      ⚠ {data.reason}
    </div>
  </foreignObject>
{/if}
```

Register in `nodeTypes` / `edgeTypes`:
```typescript
const edgeTypes = { default: DmEdge };
```

### 3. Port Handle Tooltips

In `DmNode.svelte`, add title attributes to handles showing schema info:

```svelte
<Handle
  type="target"
  position={Position.Left}
  id={`in-${port}`}
  title={portSchemaMap?.[port]?.description || port}
/>
```

Schema info can be loaded from the node registry (already fetched in P3's NodePalette) and passed through node data.

### 4. Node Status Indicators

In `DmNode.svelte`, show a badge for node resolution status:

```svelte
{#if data.isUnresolved}
  <div class="dm-node__badge dm-node__badge--error">
    Missing
  </div>
{/if}
```

Unresolved status comes from the `DataflowExecutableDetail.nodes[]` data that's already part of the `GET /dataflows/{name}` response.

### 5. Copy/Paste

```typescript
let clipboard = $state<DmFlowNode | null>(null);

function handleCopy() {
  if (!selectedNode || selectedNode.data.isVirtual) return;
  clipboard = structuredClone(selectedNode);
}

function handlePaste() {
  if (!clipboard) return;
  const existingIds = new Set(nodes.map(n => n.id));
  const newNode: DmFlowNode = {
    ...structuredClone(clipboard),
    id: generateNodeId(clipboard.data.nodeType, existingIds),
    position: {
      x: clipboard.position.x + 50,
      y: clipboard.position.y + 50,
    },
  };
  newNode.data.label = newNode.id;
  nodes = [...nodes, newNode];
  isDirty = true;
}
```

Keyboard: `Ctrl+C` → copy, `Ctrl+V` → paste.

### 6. Undo/Redo

Implement a simple snapshot-based undo stack:

```typescript
interface GraphSnapshot {
  nodes: DmFlowNode[];
  edges: DmFlowEdge[];
}

const MAX_UNDO = 50;
let undoStack = $state<GraphSnapshot[]>([]);
let redoStack = $state<GraphSnapshot[]>([]);

function pushUndo() {
  undoStack = [...undoStack.slice(-(MAX_UNDO - 1)), {
    nodes: structuredClone(nodes),
    edges: structuredClone(edges),
  }];
  redoStack = [];
}

function undo() {
  if (undoStack.length === 0) return;
  redoStack = [...redoStack, { nodes: structuredClone(nodes), edges: structuredClone(edges) }];
  const snapshot = undoStack[undoStack.length - 1];
  undoStack = undoStack.slice(0, -1);
  nodes = snapshot.nodes;
  edges = snapshot.edges;
  isDirty = true;
}

function redo() {
  if (redoStack.length === 0) return;
  undoStack = [...undoStack, { nodes: structuredClone(nodes), edges: structuredClone(edges) }];
  const snapshot = redoStack[redoStack.length - 1];
  redoStack = redoStack.slice(0, -1);
  nodes = snapshot.nodes;
  edges = snapshot.edges;
  isDirty = true;
}
```

Call `pushUndo()` before each mutation (connect, delete, add node, etc.).

Keyboard: `Ctrl+Z` → undo, `Ctrl+Shift+Z` → redo.

### 7. Theme Integration

Ensure all custom styles use CSS variables that adapt to dark/light mode:

```css
:root {
  --dm-edge-default: hsl(var(--muted-foreground));
  --dm-edge-valid: hsl(142 70% 45%);
  --dm-edge-error: hsl(0 84% 60%);
  --dm-node-bg: hsl(var(--card));
  --dm-node-border: hsl(var(--border));
  --dm-node-header: hsl(var(--muted));
}

/* SvelteFlow overrides */
:global(.svelte-flow) {
  --xy-background-color: hsl(var(--background));
  --xy-minimap-background: hsl(var(--muted));
  --xy-controls-button-background: hsl(var(--card));
  --xy-controls-button-color: hsl(var(--foreground));
}
```

### 8. Fit-to-View & Zoom Controls

Use SvelteFlow's built-in `fitView()` method and add custom toolbar buttons:

```svelte
<div class="flex gap-1">
  <Button variant="ghost" size="sm" onclick={() => fitView()}>
    Fit View
  </Button>
  <Button variant="ghost" size="sm" onclick={() => zoomTo(1)}>
    100%
  </Button>
</div>
```

---

## Backend Changes

### New endpoint: `GET /api/dataflows/{name}/validate`

Returns transpile diagnostics (schema validation + node resolution).

1. **`service.rs`**: Add `validate()` function using existing `transpile::passes::parse` + `validate_port_schemas`
2. **`handlers/dataflow.rs`**: Add `validate_dataflow` handler
3. **`handlers/mod.rs`**: Re-export
4. **`main.rs`**: Add route `.route("/api/dataflows/{name}/validate", get(handlers::validate_dataflow))`

The `TranspileDiagnostic` struct needs to derive `Serialize` if it doesn't already:

```rust
#[derive(Debug, Serialize)]
pub struct TranspileDiagnostic {
    pub yaml_id: String,
    pub node_id: String,
    pub kind: DiagnosticKind,
}
```

---

## Verification

### Cargo test

```bash
cargo test -p dm-core
```

### Svelte check

```bash
cd web && npm run check
```

### Manual tests

1. **Edge colors**: connect nodes with matching schemas → green edge
2. **Error edge**: connect incompatible ports → red edge with error tooltip
3. **Unvalidated**: connect nodes without schemas → grey edge (no error)
4. **Missing node**: node with unresolved type shows "Missing" badge
5. **Dark mode**: toggle theme → all graph elements adapt correctly
6. **Copy/paste**: select node → Ctrl+C → Ctrl+V → duplicate appears offset
7. **Undo/redo**: delete a node → Ctrl+Z → node reappears → Ctrl+Shift+Z → deleted again
8. **Fit view**: click "Fit View" → canvas fits all nodes
9. **Port tooltip**: hover over a handle → shows schema type info
