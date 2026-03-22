# P3: Full Editing — NodePalette & NodeInspector

Add node creation via a drag-and-drop palette panel, and a property inspector for editing selected node configuration.

## Prerequisites

- **P1 + P2 completed**
- Existing `GET /api/nodes` returns the list of installed nodes with `dm.json` metadata

## Scope

| Feature | Description |
|---------|-------------|
| NodePalette | Sidebar listing all installed nodes, with search and category filter |
| Drag-to-add | Drag a node from the palette onto the canvas to create a new instance |
| NodeInspector | Side panel showing selected node properties for editing |
| Auto ID generation | New nodes get unique IDs based on node type |
| Dynamic ports | Support for nodes with `dynamic_ports: true` — user can add/remove ports |
| Built-in source creation | Add timer/panel virtual nodes from the palette |

---

## Reference: Node Data Model

From `dm.json` / `Node` struct in `crates/dm-core/src/node/model.rs`:

```jsonc
{
  "id": "dm-microphone",                     // unique node identifier
  "name": "dm-microphone",                   // human-readable display name
  "description": "Audio capture from mic",
  "display": {
    "category": "Builtin/Utility",           // for palette grouping
    "tags": ["builtin", "audio"]             // for palette filtering
  },
  "ports": [
    {
      "id": "tick",
      "name": "tick",
      "direction": "input",
      "description": "Heartbeat timer",
      "required": true,
      "schema": { "type": { "name": "null" } }
    },
    ...
  ],
  "dynamic_ports": false,
  "config_schema": { ... }
}
```

**Important distinction**: `id` is the technical identifier, `name` is the display name. The palette should show `name` (fallback to `id`) and use `id` for YAML generation.

---

## Frontend Changes

### File Structure Update

```
web/src/routes/dataflows/[id]/components/
├── GraphEditorTab.svelte          # MODIFY — add palette/inspector layout
└── graph/
    ├── DmNode.svelte              # MODIFY — selection styling
    ├── NodePalette.svelte         # NEW
    ├── NodeInspector.svelte       # NEW
    ├── types.ts                   # MODIFY — add palette types
    ├── yaml-graph.ts              # MODIFY — add node creation helpers
    └── auto-layout.ts             # existing
```

### 1. `graph/types.ts` — Palette types

```typescript
// A node template from dm.json, used in the palette
export interface NodeTemplate {
  id: string;            // dm.json id
  name: string;          // dm.json name (display)
  description: string;
  category: string;      // display.category
  tags: string[];        // display.tags
  inputs: PalettePort[];
  outputs: PalettePort[];
  dynamicPorts: boolean;
  configSchema?: Record<string, unknown>;
}

export interface PalettePort {
  id: string;
  name: string;
  description: string;
  required: boolean;
}

// Virtual node templates (not from API)
export const VIRTUAL_TEMPLATES: NodeTemplate[] = [
  {
    id: '__dora_timer',
    name: 'Timer',
    description: 'Dora built-in periodic timer',
    category: 'Dora/Built-in',
    tags: ['builtin', 'timer'],
    inputs: [],
    outputs: [{ id: 'tick', name: 'tick', description: 'Periodic tick', required: true }],
    dynamicPorts: false,
  },
];
```

### 2. `graph/NodePalette.svelte`

Searchable, categorized list of available nodes:

```svelte
<script lang="ts">
  import { get } from '$lib/api';
  import { onMount } from 'svelte';
  import { Search } from 'lucide-svelte';
  import { Input } from '$lib/components/ui/input/index.js';
  import type { NodeTemplate, PalettePort } from './types';
  import { VIRTUAL_TEMPLATES } from './types';

  let { onDragStart } = $props<{
    onDragStart: (template: NodeTemplate) => void;
  }>();

  let templates = $state<NodeTemplate[]>([]);
  let searchQuery = $state('');
  let selectedCategory = $state<string | null>(null);

  // Extract unique categories
  let categories = $derived(() => {
    const cats = new Set(templates.map(t => t.category).filter(Boolean));
    return ['All', ...Array.from(cats).sort()];
  });

  // Filter templates
  let filteredTemplates = $derived(() => {
    let result = templates;
    if (selectedCategory && selectedCategory !== 'All') {
      result = result.filter(t => t.category === selectedCategory);
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter(t =>
        t.name.toLowerCase().includes(q) ||
        t.id.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q) ||
        t.tags.some(tag => tag.toLowerCase().includes(q))
      );
    }
    return result;
  });

  onMount(async () => {
    try {
      const nodes: any[] = await get('/nodes');
      templates = [
        ...VIRTUAL_TEMPLATES,
        ...nodes.map(n => ({
          id: n.id,
          name: n.name || n.id,
          description: n.description || '',
          category: n.display?.category || 'Uncategorized',
          tags: n.display?.tags || [],
          inputs: (n.ports || [])
            .filter((p: any) => p.direction === 'input')
            .map((p: any) => ({
              id: p.id,
              name: p.name || p.id,
              description: p.description || '',
              required: p.required ?? true,
            })),
          outputs: (n.ports || [])
            .filter((p: any) => p.direction === 'output')
            .map((p: any) => ({
              id: p.id,
              name: p.name || p.id,
              description: p.description || '',
              required: p.required ?? true,
            })),
          dynamicPorts: n.dynamic_ports || false,
          configSchema: n.config_schema || undefined,
        })),
      ];
    } catch (e: any) {
      console.error('Failed to load nodes:', e);
    }
  });

  function handleDragStart(e: DragEvent, template: NodeTemplate) {
    e.dataTransfer?.setData('application/dm-node', JSON.stringify(template));
    onDragStart(template);
  }
</script>

<div class="w-64 border-r flex flex-col h-full bg-card">
  <!-- Search -->
  <div class="p-3 border-b">
    <div class="relative">
      <Search class="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
      <Input
        bind:value={searchQuery}
        placeholder="Search nodes..."
        class="pl-8 h-9"
      />
    </div>
  </div>

  <!-- Categories -->
  <div class="flex gap-1 p-3 flex-wrap border-b">
    {#each categories() as cat}
      <button
        class="text-xs px-2 py-1 rounded-full transition-colors"
        class:bg-primary={selectedCategory === cat || (!selectedCategory && cat === 'All')}
        class:text-primary-foreground={selectedCategory === cat || (!selectedCategory && cat === 'All')}
        class:bg-muted={selectedCategory !== cat && !(cat === 'All' && !selectedCategory)}
        onclick={() => selectedCategory = cat === 'All' ? null : cat}
      >
        {cat}
      </button>
    {/each}
  </div>

  <!-- Node list -->
  <div class="flex-1 overflow-y-auto p-2 space-y-1">
    {#each filteredTemplates() as template}
      <div
        draggable="true"
        ondragstart={(e) => handleDragStart(e, template)}
        class="p-2.5 rounded-md border border-transparent hover:border-border
               hover:bg-muted/50 cursor-grab active:cursor-grabbing transition-colors"
      >
        <div class="font-medium text-sm font-mono">
          {template.name}
        </div>
        {#if template.name !== template.id}
          <div class="text-xs text-muted-foreground font-mono mt-0.5">
            {template.id}
          </div>
        {/if}
        {#if template.description}
          <div class="text-xs text-muted-foreground mt-1 line-clamp-2">
            {template.description}
          </div>
        {/if}
        <div class="flex gap-1 mt-1.5 flex-wrap">
          {#each template.tags.slice(0, 3) as tag}
            <span class="text-[10px] px-1.5 py-0.5 bg-muted rounded text-muted-foreground">
              {tag}
            </span>
          {/each}
        </div>
      </div>
    {/each}

    {#if filteredTemplates().length === 0}
      <div class="text-sm text-muted-foreground text-center py-6">
        No matching nodes
      </div>
    {/if}
  </div>
</div>
```

### 3. `graph/NodeInspector.svelte`

Side panel for viewing/editing selected node properties:

```svelte
<script lang="ts">
  import { Input } from '$lib/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import { Plus, Trash2, X } from 'lucide-svelte';
  import type { DmFlowNode } from './types';

  let {
    node,
    onClose,
    onNodeUpdate,
    onAddPort,
    onRemovePort,
  } = $props<{
    node: DmFlowNode;
    onClose: () => void;
    onNodeUpdate: (id: string, data: Partial<DmFlowNode['data']>) => void;
    onAddPort?: (nodeId: string, direction: 'input' | 'output', portId: string) => void;
    onRemovePort?: (nodeId: string, direction: 'input' | 'output', portId: string) => void;
  }>();

  let newInputPort = $state('');
  let newOutputPort = $state('');
</script>

<div class="w-72 border-l flex flex-col h-full bg-card">
  <!-- Header -->
  <div class="p-3 border-b flex items-center justify-between">
    <h3 class="font-semibold text-sm">Node Inspector</h3>
    <button onclick={onClose} class="p-1 rounded hover:bg-muted">
      <X class="size-4" />
    </button>
  </div>

  <div class="flex-1 overflow-y-auto p-3 space-y-4">
    <!-- Node Identity -->
    <div>
      <label class="text-xs text-muted-foreground font-medium">Instance ID</label>
      <div class="text-sm font-mono mt-1">{node.id}</div>
    </div>

    <div>
      <label class="text-xs text-muted-foreground font-medium">Node Type</label>
      <div class="text-sm font-mono mt-1">{node.data.nodeType}</div>
    </div>

    {#if node.data.isVirtual}
      <Badge variant="outline">Virtual Node</Badge>
    {/if}

    <!-- Input Ports -->
    <div>
      <div class="flex items-center justify-between mb-1.5">
        <label class="text-xs text-muted-foreground font-medium">
          Input Ports ({node.data.inputs.length})
        </label>
      </div>
      {#each node.data.inputs as port}
        <div class="flex items-center gap-2 py-1">
          <div class="w-2 h-2 rounded-full bg-blue-500"></div>
          <span class="text-xs font-mono flex-1">{port}</span>
          {#if onRemovePort && !node.data.isVirtual}
            <button
              class="p-0.5 rounded hover:bg-muted"
              onclick={() => onRemovePort?.(node.id, 'input', port)}
            >
              <Trash2 class="size-3 text-muted-foreground" />
            </button>
          {/if}
        </div>
      {/each}

      <!-- Add port (for dynamic_ports nodes) -->
      {#if onAddPort && !node.data.isVirtual}
        <div class="flex gap-1 mt-2">
          <Input
            bind:value={newInputPort}
            placeholder="port_name"
            class="h-7 text-xs font-mono"
          />
          <Button
            variant="outline"
            size="sm"
            class="h-7 px-2"
            disabled={!newInputPort.trim()}
            onclick={() => {
              onAddPort?.(node.id, 'input', newInputPort.trim());
              newInputPort = '';
            }}
          >
            <Plus class="size-3" />
          </Button>
        </div>
      {/if}
    </div>

    <!-- Output Ports -->
    <div>
      <div class="flex items-center justify-between mb-1.5">
        <label class="text-xs text-muted-foreground font-medium">
          Output Ports ({node.data.outputs.length})
        </label>
      </div>
      {#each node.data.outputs as port}
        <div class="flex items-center gap-2 py-1">
          <div class="w-2 h-2 rounded-full bg-green-500"></div>
          <span class="text-xs font-mono flex-1">{port}</span>
          {#if onRemovePort && !node.data.isVirtual}
            <button
              class="p-0.5 rounded hover:bg-muted"
              onclick={() => onRemovePort?.(node.id, 'output', port)}
            >
              <Trash2 class="size-3 text-muted-foreground" />
            </button>
          {/if}
        </div>
      {/each}

      {#if onAddPort && !node.data.isVirtual}
        <div class="flex gap-1 mt-2">
          <Input
            bind:value={newOutputPort}
            placeholder="port_name"
            class="h-7 text-xs font-mono"
          />
          <Button
            variant="outline"
            size="sm"
            class="h-7 px-2"
            disabled={!newOutputPort.trim()}
            onclick={() => {
              onAddPort?.(node.id, 'output', newOutputPort.trim());
              newOutputPort = '';
            }}
          >
            <Plus class="size-3" />
          </Button>
        </div>
      {/if}
    </div>
  </div>
</div>
```

### 4. `graph/yaml-graph.ts` — Node creation helpers

```typescript
// Generate a unique instance ID for a new node
export function generateNodeId(
  nodeTypeId: string,
  existingIds: Set<string>
): string {
  // For most nodes, use the node type id directly if not taken
  if (!existingIds.has(nodeTypeId)) return nodeTypeId;
  // Otherwise append a numeric suffix
  let i = 1;
  while (existingIds.has(`${nodeTypeId}-${i}`)) i++;
  return `${nodeTypeId}-${i}`;
}

// Create a new DmFlowNode from a NodeTemplate
export function createNodeFromTemplate(
  template: NodeTemplate,
  position: { x: number; y: number },
  existingIds: Set<string>
): DmFlowNode {
  const id = generateNodeId(template.id, existingIds);

  // Handle virtual timer node specially
  if (template.id === '__dora_timer') {
    return {
      id: `__virtual_dora_timer_millis_1000`,
      type: 'dmNode',
      position,
      data: {
        label: 'Timer 1000ms',
        nodeType: 'dora/timer/millis/1000',
        inputs: [],
        outputs: ['tick'],
        isVirtual: true,
        virtualKind: 'timer',
      },
    };
  }

  return {
    id,
    type: 'dmNode',
    position,
    data: {
      label: id,
      nodeType: template.id,
      inputs: template.inputs.map(p => p.id),
      outputs: template.outputs.map(p => p.id),
    },
  };
}
```

### 5. `GraphEditorTab.svelte` — Layout with palette + inspector

```svelte
<div class="h-full flex min-h-0 w-full">
  <!-- Left: Node Palette -->
  <NodePalette onDragStart={handlePaletteDragStart} />

  <!-- Center: Graph Canvas -->
  <div class="flex-1 flex flex-col min-h-0">
    <div class="flex justify-end p-2">
      <Button ...>Save</Button>
    </div>
    <div class="flex-1 min-h-0 relative border rounded-md shadow-sm overflow-hidden"
         ondrop={handleDrop}
         ondragover={(e) => e.preventDefault()}>
      <SvelteFlow ...>
        <!-- ... -->
      </SvelteFlow>
    </div>
  </div>

  <!-- Right: Node Inspector (conditional) -->
  {#if selectedNode}
    <NodeInspector
      node={selectedNode}
      onClose={() => selectedNode = null}
      onNodeUpdate={handleNodeUpdate}
      onAddPort={handleAddPort}
      onRemovePort={handleRemovePort}
    />
  {/if}
</div>
```

**Drop handler** for palette drag-and-drop:

```typescript
let draggedTemplate = $state<NodeTemplate | null>(null);

function handlePaletteDragStart(template: NodeTemplate) {
  draggedTemplate = template;
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  if (!draggedTemplate) return;

  // Convert screen coordinates to canvas coordinates
  // (requires SvelteFlow's screenToFlowPosition)
  const position = screenToFlowPosition({ x: e.clientX, y: e.clientY });

  const existingIds = new Set(nodes.map(n => n.id));
  const newNode = createNodeFromTemplate(draggedTemplate, position, existingIds);
  nodes = [...nodes, newNode];
  isDirty = true;
  draggedTemplate = null;
}
```

**Selection handler** for inspector:

```typescript
let selectedNode = $state<DmFlowNode | null>(null);

function onSelectionChange({ nodes: selectedNodes }) {
  selectedNode = selectedNodes.length === 1 ? selectedNodes[0] : null;
}
```

**Port management handlers:**

```typescript
function handleAddPort(nodeId: string, direction: 'input' | 'output', portId: string) {
  nodes = nodes.map(n => {
    if (n.id !== nodeId) return n;
    const data = { ...n.data };
    if (direction === 'input') data.inputs = [...data.inputs, portId];
    else data.outputs = [...data.outputs, portId];
    return { ...n, data };
  });
  isDirty = true;
}

function handleRemovePort(nodeId: string, direction: 'input' | 'output', portId: string) {
  nodes = nodes.map(n => {
    if (n.id !== nodeId) return n;
    const data = { ...n.data };
    if (direction === 'input') data.inputs = data.inputs.filter(p => p !== portId);
    else data.outputs = data.outputs.filter(p => p !== portId);
    return { ...n, data };
  });
  // Remove edges connected to the removed port
  const handleId = direction === 'input' ? `in-${portId}` : `out-${portId}`;
  edges = edges.filter(e => {
    if (direction === 'input') return !(e.target === nodeId && e.targetHandle === handleId);
    return !(e.source === nodeId && e.sourceHandle === handleId);
  });
  isDirty = true;
}
```

---

## Backend Changes

No new backend API needed — `GET /api/nodes` already returns full node metadata including `ports`, `display.category`, `display.tags`, and `dynamic_ports`.

---

## Verification

### Svelte check

```bash
cd web && npm run check
```

### Manual tests

1. **Palette renders**: left sidebar shows all installed nodes grouped by category
2. **Search**: type "audio" → filters to matching nodes
3. **Category filter**: click "Builtin/Utility" → shows only matching nodes
4. **Drag to add**: drag `dm-microphone` from palette → drop on canvas → new node appears with correct ports
5. **Auto ID**: drag second `dm-microphone` → ID becomes `dm-microphone-1`
6. **Virtual timer**: drag "Timer" → creates virtual timer node with dashed border
7. **Inspector**: click a node → right panel shows node details
8. **Add port**: in inspector, add a new output port → port appears on node, can be connected
9. **Remove port**: remove a port → connected edges also removed
10. **Save round-trip**: add node + connect + save → refresh → all persisted
11. **YAML sync**: add node in graph → save → YAML tab shows new node entry
