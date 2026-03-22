# P2: Editable Canvas — Topology Mutations

Build on P1 to support interactive topology editing: connecting/disconnecting ports, deleting nodes, and serializing graph changes back to YAML.

## Prerequisites

- **P1 completed** — read-only canvas, `view.json` persistence, `yamlToGraph()` working.

## Scope

| Feature | Description |
|---------|-------------|
| Connect | Drag from output handle → input handle to create an edge |
| Disconnect | Click edge → delete key, or edge context menu |
| Delete node | Select node → delete key |
| Graph → YAML | Serialize graph back to valid `dataflow.yml` |
| Bidirectional sync | Graph tab ↔ YAML tab share the same data |

---

## Frontend Changes

### 1. `graph/yaml-graph.ts` — Add `graphToYaml()`

This is the reverse of `yamlToGraph()`. It takes the current SvelteFlow state and produces a valid YAML string.

```typescript
export function graphToYaml(
  nodes: DmFlowNode[],
  edges: DmFlowEdge[],
  originalYamlStr?: string   // for preserving fields like config/env/widgets
): string {
  // Parse original YAML to preserve non-graph fields
  let original: { nodes?: YamlNode[] } = {};
  if (originalYamlStr) {
    try { original = YAML.parse(originalYamlStr) || {}; } catch {}
  }
  const originalNodeMap = new Map<string, YamlNode>();
  for (const n of original.nodes || []) {
    originalNodeMap.set(n.id, n);
  }

  // Build edge index: target → [{inputPort, sourceStr}]
  const edgeIndex = new Map<string, { port: string; source: string }[]>();
  for (const edge of edges) {
    // Skip edges from virtual nodes — reconstruct original source string
    const sourceStr = resolveEdgeToInputValue(edge, nodes);
    if (!sourceStr) continue;
    const inputPort = edge.targetHandle?.replace('in-', '') || '';
    const arr = edgeIndex.get(edge.target) || [];
    arr.push({ port: inputPort, source: sourceStr });
    edgeIndex.set(edge.target, arr);
  }

  // Build YAML nodes (skip virtual nodes)
  const yamlNodes: YamlNode[] = [];
  for (const node of nodes) {
    if (node.data.isVirtual) continue;

    const orig = originalNodeMap.get(node.id);
    const inputs: Record<string, string> = {};
    for (const { port, source } of edgeIndex.get(node.id) || []) {
      inputs[port] = source;
    }

    yamlNodes.push({
      id: node.id,
      node: node.data.nodeType,
      inputs: Object.keys(inputs).length > 0 ? inputs : undefined,
      outputs: node.data.outputs.length > 0 ? node.data.outputs : undefined,
      // Preserve original config/env/widgets
      config: orig?.config,
      env: orig?.env,
      widgets: orig?.widgets,
    });
  }

  return YAML.stringify({ nodes: yamlNodes });
}

// Convert a SvelteFlow edge back to the YAML input value string
function resolveEdgeToInputValue(
  edge: DmFlowEdge,
  nodes: DmFlowNode[]
): string | null {
  const sourceNode = nodes.find(n => n.id === edge.source);
  if (!sourceNode) return null;

  if (sourceNode.data.isVirtual) {
    if (sourceNode.data.virtualKind === 'timer') {
      // nodeType stores the raw dora source, e.g. "dora/timer/millis/2000"
      return sourceNode.data.nodeType;
    }
    if (sourceNode.data.virtualKind === 'panel') {
      const port = edge.sourceHandle?.replace('out-', '') || '';
      return `panel/${port}`;
    }
    return null;
  }

  // Regular node-to-node edge
  const outputPort = edge.sourceHandle?.replace('out-', '') || '';
  return `${sourceNode.id}/${outputPort}`;
}
```

**Key design points:**
- Virtual node edges are resolved back to their original YAML strings (`dora/timer/millis/2000`, `panel/device_id`)
- Original `config`, `env`, `widgets` are preserved from the original YAML (the graph editor doesn't edit these)
- Comments are NOT preserved (see P1 discussion — acceptable for now)

### 2. `GraphEditorTab.svelte` — Enable mutations

Modify the existing component from P1:

```svelte
<script lang="ts">
  // ... existing imports ...
  import { yamlToGraph, graphToYaml, buildViewJson } from './graph/yaml-graph';

  let {
    dataflowName,
    yamlStr = '',
    viewJson = {} as ViewJson,
    onYamlUpdated,            // NEW: callback to propagate changes to parent
  } = $props<{
    dataflowName: string;
    yamlStr?: string;
    viewJson?: ViewJson;
    onYamlUpdated?: (yaml: string) => void;    // NEW
  }>();

  // Track the original YAML for config preservation during serialization
  let originalYaml = $state(yamlStr);
  let isDirty = $state(false);

  // --- Event handlers for graph mutations ---

  function onConnect(connection) {
    // Add new edge
    const newEdge = {
      id: `e-${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
      source: connection.source,
      target: connection.target,
      sourceHandle: connection.sourceHandle,
      targetHandle: connection.targetHandle,
    };
    edges = [...edges, newEdge];
    isDirty = true;
  }

  function onEdgesDelete(deletedEdges) {
    const deleteIds = new Set(deletedEdges.map(e => e.id));
    edges = edges.filter(e => !deleteIds.has(e.id));
    isDirty = true;
  }

  function onNodesDelete(deletedNodes) {
    const deleteIds = new Set(deletedNodes.map(n => n.id));

    // Don't allow deleting virtual nodes from keyboard
    if (deletedNodes.some(n => n.data.isVirtual)) return;

    nodes = nodes.filter(n => !deleteIds.has(n.id));
    // Also remove all edges connected to deleted nodes
    edges = edges.filter(e => !deleteIds.has(e.source) && !deleteIds.has(e.target));
    isDirty = true;
  }

  // --- Save: serialize graph → YAML + view.json ---

  async function saveAll() {
    isSaving = true;
    try {
      // 1. Serialize graph to YAML
      const newYaml = graphToYaml(nodes, edges, originalYaml);

      // 2. Save YAML
      await post(`/dataflows/${dataflowName}`, { yaml: newYaml });

      // 3. Save view.json
      const view = buildViewJson(nodes);
      await post(`/dataflows/${dataflowName}/view`, view);

      // 4. Notify parent
      originalYaml = newYaml;
      onYamlUpdated?.(newYaml);
      isDirty = false;

      toast.success('Saved');
    } catch (e: any) {
      toast.error(`Save failed: ${e.message}`);
    } finally {
      isSaving = false;
    }
  }
</script>

<!-- Template: add event handlers to SvelteFlow -->
<SvelteFlow
  {nodes}
  {edges}
  {nodeTypes}
  fitView
  onconnect={onConnect}
  onedgesdelete={onEdgesDelete}
  onodesdelete={onNodesDelete}
>
  ...
</SvelteFlow>

<!-- Update save button to show dirty state -->
<Button
  variant={isDirty ? 'default' : 'outline'}
  size="sm"
  disabled={isSaving}
  onclick={saveAll}
>
  <Save class="mr-2 size-4" />
  {isSaving ? 'Saving...' : isDirty ? 'Save changes' : 'Save layout'}
</Button>
```

### 3. `+page.svelte` — Bidirectional sync

Update the parent page to handle YAML updates from the graph editor:

```svelte
<GraphEditorTab
  {dataflowName}
  yamlStr={dataflow.yaml || ''}
  viewJson={dataflow.view || {}}
  onYamlUpdated={(newYaml) => {
    if (dataflow) dataflow.yaml = newYaml;
  }}
/>
```

When user edits YAML in the YAML tab and saves, the `dataflow.yaml` state variable updates. When they switch back to Graph tab, the `$effect` in `GraphEditorTab` re-parses the YAML.

### 4. Connection validation (basic)

Add a validation callback to `SvelteFlow` to prevent invalid connections:

```typescript
function isValidConnection(connection): boolean {
  // Prevent self-loops
  if (connection.source === connection.target) return false;

  // Prevent duplicate connections to the same input port
  const hasExisting = edges.some(
    e => e.target === connection.target && e.targetHandle === connection.targetHandle
  );
  if (hasExisting) return false;

  // Only allow output → input connections (not input → input)
  if (!connection.sourceHandle?.startsWith('out-')) return false;
  if (!connection.targetHandle?.startsWith('in-')) return false;

  return true;
}
```

### 5. Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Delete` / `Backspace` | Delete selected nodes/edges |
| `Ctrl+S` | Save all changes |
| `Ctrl+A` | Select all nodes |

Implement via SvelteFlow's built-in `deleteKeyCode` and custom `keydown` handler for `Ctrl+S`.

---

## Backend Changes

No new backend changes — P1's `view.json` API and existing `POST /api/dataflows/{name}` (save YAML) are sufficient.

---

## Verification

### Svelte check

```bash
cd web && npm run check
```

### Manual tests

1. **Connect**: drag from `dm-microphone/audio` output handle to `dora-vad/audio` input handle → edge appears
2. **Disconnect**: click edge → press Delete → edge removed
3. **Delete node**: select `screen-live` → press Delete → node and all connected edges removed
4. **Save**: click "Save changes" → refresh page → topology is preserved
5. **YAML sync**: save from graph → switch to YAML tab → new connections visible as `inputs:` entries
6. **YAML tab edit**: edit YAML to add a new input → save → switch to Graph → new edge appears
7. **Validation**: try to connect output→output → should be blocked
8. **Virtual nodes**: deleting a virtual node should be blocked
