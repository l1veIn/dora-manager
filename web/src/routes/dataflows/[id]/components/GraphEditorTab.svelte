<script lang="ts">
    import {
        SvelteFlow,
        Controls,
        MiniMap,
        Background,
        BackgroundVariant,
    } from '@xyflow/svelte';
    import '@xyflow/svelte/dist/style.css';
    import { Button } from '$lib/components/ui/button/index.js';
    import { Maximize2 } from 'lucide-svelte';
    import { mode } from 'mode-watcher';
    import DmNode from './graph/DmNode.svelte';
    import { yamlToGraph } from './graph/yaml-graph';
    import type { ViewJson, DmFlowNode, DmFlowEdge } from './graph/types';

    let {
        dataflowName,
        yamlStr = '',
        viewJson = {} as ViewJson,
    }: {
        dataflowName: string;
        yamlStr?: string;
        viewJson?: ViewJson;
    } = $props();

    const nodeTypes: any = { dmNode: DmNode };

    let nodes = $state<DmFlowNode[]>([]);
    let edges = $state<DmFlowEdge[]>([]);

    $effect(() => {
        const result = yamlToGraph(yamlStr, viewJson);
        nodes = result.nodes;
        edges = result.edges;
    });

    let colorMode = $derived(
        mode.current === 'dark' ? 'dark' : ('light' as 'dark' | 'light'),
    );
</script>

<div class="h-full flex flex-col min-h-0 w-full">
    <!-- Toolbar -->
    <div class="flex items-center justify-end gap-2 mb-3">
        <Button variant="default" size="sm" href="/dataflows/{dataflowName}/editor">
            <Maximize2 class="mr-2 size-4" />
            Open Editor
        </Button>
    </div>

    <!-- Read-only canvas -->
    <div
        class="flex-1 min-h-0 relative border rounded-md shadow-sm overflow-hidden graph-container"
    >
        <SvelteFlow
            bind:nodes
            bind:edges
            {nodeTypes}
            fitView
            colorMode={colorMode}
            proOptions={{ hideAttribution: true }}
            nodesDraggable={false}
            nodesConnectable={false}
            elementsSelectable={false}
            deleteKey={null}
        >
            <Controls />
            <MiniMap />
            <Background variant={BackgroundVariant.Dots} />
        </SvelteFlow>
    </div>
</div>

<style>
    .graph-container {
        min-height: 400px;
    }

    /* ── Light mode canvas ── */
    .graph-container :global(.svelte-flow) {
        --xy-background-color: #f8fafc;
        --xy-minimap-background-color: #f1f5f9;
        --xy-node-border-radius: 10px;
    }

    /* ── Dark mode canvas ── */
    .graph-container :global(.svelte-flow.dark) {
        --xy-background-color: #0f172a;
        --xy-minimap-background-color: #1e293b;
    }

    .graph-container :global(.svelte-flow__minimap) {
        border-radius: 8px;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    }

    /* ── Edge styling ── */
    .graph-container :global(.svelte-flow__edge-path) {
        stroke-width: 2;
        stroke: #94a3b8;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__edge-path) {
        stroke: #64748b;
    }

    /* ── Handle styling ── */
    .graph-container :global(.svelte-flow__handle) {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        border: 2px solid #cbd5e1;
        background: #ffffff;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__handle) {
        border-color: #475569;
        background: #1e293b;
    }

    /* ── Dark controls ── */
    .graph-container :global(.svelte-flow.dark .svelte-flow__controls) {
        background: #1e293b;
        border-color: #334155;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__controls button) {
        background: #1e293b;
        color: #e2e8f0;
        border-color: #334155;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__controls button:hover) {
        background: #334155;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__controls button svg) {
        fill: #e2e8f0;
    }
</style>
