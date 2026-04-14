<script lang="ts">
    import { page } from '$app/state';
    import { onMount } from 'svelte';
    import { get, post } from '$lib/api';
    import { goto } from '$app/navigation';
    import { toast } from 'svelte-sonner';
    import {
        SvelteFlow,
        Controls,
        MiniMap,
        Background,
        BackgroundVariant,
    } from '@xyflow/svelte';
    import '@xyflow/svelte/dist/style.css';
    import { Button } from '$lib/components/ui/button/index.js';
    import {
        Save,
        ArrowLeft,
        PanelLeft,
        PanelRight,
        Undo2,
        Redo2,
        Plus,
        Sun,
        Moon,
        Languages,
        LayoutGrid,
    } from 'lucide-svelte';
    import { mode, toggleMode } from 'mode-watcher';
    import DmNode from '../components/graph/DmNode.svelte';
    import DmEdge from '../components/graph/DmEdge.svelte';
    import ContextMenu from '../components/graph/ContextMenu.svelte';
    import NodePalette from '../components/graph/NodePalette.svelte';
    import NodeInspector from '../components/graph/NodeInspector.svelte';
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { t, locale } from "svelte-i18n";
    import {
        yamlToGraph,
        buildViewJson,
        graphToYaml,
        createNodeFromPalette,
    } from '../components/graph/yaml-graph';
    import { applyDagreLayout } from '../components/graph/auto-layout';
    import type {
        ViewJson,
        DmFlowNode,
        DmFlowEdge,
    } from '../components/graph/types';

    let dataflowName = $derived(page.params.id as string);
    let dataflow = $state<any>(null);
    let loading = $state(true);

    const nodeTypes: any = { dmNode: DmNode };
    const edgeTypes: any = { default: DmEdge };

    let nodes = $state<DmFlowNode[]>([]);
    let edges = $state<DmFlowEdge[]>([]);
    let isSaving = $state(false);
    let isDirty = $state(false);
    let lastYaml = $state('');
    let selectedNode = $state<DmFlowNode | null>(null);
    let showPalette = $state(false);
    let showInspector = $state(false);

    // ── Undo/Redo ──
    interface Snapshot {
        nodes: DmFlowNode[];
        edges: DmFlowEdge[];
    }
    const MAX_UNDO = 30;
    let undoStack = $state<Snapshot[]>([]);
    let redoStack = $state<Snapshot[]>([]);

    function deepClone<T>(val: T): T {
        return JSON.parse(JSON.stringify(val));
    }

    function pushUndo() {
        undoStack = [
            ...undoStack.slice(-(MAX_UNDO - 1)),
            { nodes: deepClone(nodes), edges: deepClone(edges) },
        ];
        redoStack = [];
    }

    function undo() {
        if (undoStack.length === 0) return;
        redoStack = [
            ...redoStack,
            { nodes: deepClone(nodes), edges: deepClone(edges) },
        ];
        const prev = undoStack[undoStack.length - 1];
        undoStack = undoStack.slice(0, -1);
        nodes = prev.nodes;
        edges = prev.edges;
        isDirty = true;
    }

    function redo() {
        if (redoStack.length === 0) return;
        undoStack = [
            ...undoStack,
            { nodes: deepClone(nodes), edges: deepClone(edges) },
        ];
        const next = redoStack[redoStack.length - 1];
        redoStack = redoStack.slice(0, -1);
        nodes = next.nodes;
        edges = next.edges;
        isDirty = true;
    }

    // ── Auto-Layout ──
    function applyAutoLayout() {
        if (nodes.length === 0) return;
        pushUndo();
        const layouted = applyDagreLayout(
            nodes.map(n => ({ ...n, position: { ...n.position } })) as DmFlowNode[],
            edges.map(e => ({ ...e })) as DmFlowEdge[],
        );
        nodes = layouted.nodes;
        edges = layouted.edges;
        isDirty = true;
    }

    // ── Context Menu ──
    let contextMenu = $state<{
        x: number;
        y: number;
        type: 'pane' | 'node' | 'edge' | null;
        visible: boolean;
        targetId: string | null;
    }>({
        x: 0,
        y: 0,
        type: null,
        visible: false,
        targetId: null,
    });

    function onpanecontextmenu({ event: e }: { event: MouseEvent }) {
        e.preventDefault();
        contextMenu = { x: e.clientX, y: e.clientY, type: 'pane', visible: true, targetId: null };
    }

    function onnodecontextmenu({ event: e, node }: { event: MouseEvent; node: any }) {
        e.preventDefault();
        contextMenu = { x: e.clientX, y: e.clientY, type: 'node', visible: true, targetId: node.id };
    }

    function onedgecontextmenu({ event: e, edge }: { event: MouseEvent; edge: any }) {
        e.preventDefault();
        contextMenu = { x: e.clientX, y: e.clientY, type: 'edge', visible: true, targetId: edge.id };
    }

    function handleContextMenuAction(action: string) {
        if (action === 'addNode') {
            createPosition = { x: contextMenu.x, y: contextMenu.y };
            showPalette = true;
        } else if (action === 'selectAll') {
            nodes = nodes.map(n => ({ ...n, selected: true }));
            edges = edges.map(e => ({ ...e, selected: true }));
        } else if (action === 'autoLayout') {
            applyAutoLayout();
        } else if (action === 'duplicate' && contextMenu.targetId) {
            const node = nodes.find(n => n.id === contextMenu.targetId);
            if (node) duplicateNode(node);
        } else if (action === 'deleteNode' && contextMenu.targetId) {
            ondelete({ nodes: [{ id: contextMenu.targetId }] });
        } else if (action === 'deleteEdge' && contextMenu.targetId) {
            ondelete({ edges: [{ id: contextMenu.targetId }] });
        } else if (action === 'inspect' && contextMenu.targetId) {
            const node = nodes.find(n => n.id === contextMenu.targetId);
            if (node) {
                selectedNode = node as DmFlowNode;
                showInspector = true;
            }
        }
    }

    // ── Duplicate ──
    function duplicateNode(nodeToDuplicate: DmFlowNode | null = selectedNode) {
        if (!nodeToDuplicate || nodeToDuplicate.data.isVirtual) return;
        pushUndo();
        const existingIds = new Set(nodes.map((n) => n.id));
        const cloned = deepClone(nodeToDuplicate);
        const base = (cloned.data.nodeType as string).split('/').pop() || 'node';
        let id = base;
        let i = 1;
        while (existingIds.has(id)) {
            id = `${base}-${i}`;
            i++;
        }
        cloned.id = id;
        cloned.data = { ...cloned.data, label: id };
        cloned.position = {
            x: cloned.position.x + 60,
            y: cloned.position.y + 60,
        };
        cloned.selected = false;
        nodes = [...nodes, cloned];
        isDirty = true;
        toast.info(`Duplicated node ${id}`);
    }

    // ── Load data ──
    async function loadDataflow() {
        loading = true;
        try {
            const res = await get(`/dataflows/${dataflowName}`);
            dataflow = res;
            const result = yamlToGraph(
                dataflow.yaml || '',
                dataflow.view || {},
            );
            nodes = result.nodes;
            edges = result.edges;
            lastYaml = dataflow.yaml || '';
            isDirty = false;
            undoStack = [];
            redoStack = [];
        } catch (e: any) {
            toast.error(`Failed to load: ${e.message}`);
            goto(`/dataflows/${dataflowName}`);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadDataflow();
    });

    let colorMode = $derived(
        mode.current === 'dark' ? 'dark' : ('light' as 'dark' | 'light'),
    );

    // ── Node Rename & Config ──
    function handleRenameNode(oldId: string, newId: string) {
        pushUndo();
        nodes = nodes.map(n => {
            if (n.id === oldId) {
                return { ...n, id: newId, data: { ...n.data, label: newId } };
            }
            return n;
        });
        
        edges = edges.map(e => {
            let changed = false;
            let updated = { ...e };
            if (updated.source === oldId) { updated.source = newId; changed = true; }
            if (updated.target === oldId) { updated.target = newId; changed = true; }
            if (changed) {
                updated.id = `e-${updated.source}-${(updated.sourceHandle || '').replace('out-', '')}-${updated.target}-${(updated.targetHandle || '').replace('in-', '')}`;
                return updated;
            }
            return e;
        });

        if (selectedNode?.id === oldId) {
            selectedNode = nodes.find(n => n.id === newId) as DmFlowNode;
        }
        isDirty = true;
    }

    function handleUpdateConfig(newConfig: any) {
        if (!selectedNode) return;
        // deliberate: omit pushUndo() to avoid flooding history with config keystrokes/slider drags
        nodes = nodes.map(n => {
            if (n.id === selectedNode?.id) {
                return { ...n, data: { ...n.data, config: JSON.parse(JSON.stringify(newConfig)) } };
            }
            return n;
        });
        
        if (selectedNode) {
            selectedNode = nodes.find(n => n.id === selectedNode?.id) as DmFlowNode;
        }
        isDirty = true;
    }

    // ── Connection ──
    function isValidConnection(connection: any) {
        if (connection.source === connection.target) return false;

        // Prevent multiple incoming edges to the same port
        // Dataflow typically has max in-degree of 1
        const hasIncoming = edges.some(
            (e) => e.target === connection.target && e.targetHandle === connection.targetHandle
        );
        if (hasIncoming) return false;

        // Prevent exact duplicate connections
        const isDuplicate = edges.some(
            (e) => e.source === connection.source && 
                   e.target === connection.target &&
                   e.sourceHandle === connection.sourceHandle &&
                   e.targetHandle === connection.targetHandle
        );
        if (isDuplicate) return false;

        return true;
    }

    function onconnect(connection: any) {
        if (!isValidConnection(connection)) return;
        pushUndo();
        const newEdge: DmFlowEdge = {
            id: `e-${connection.source}-${(connection.sourceHandle || '').replace('out-', '')}-${connection.target}-${(connection.targetHandle || '').replace('in-', '')}`,
            source: connection.source,
            target: connection.target,
            sourceHandle: connection.sourceHandle,
            targetHandle: connection.targetHandle,
            type: 'default',
        };
        edges = [...edges, newEdge];
        isDirty = true;
    }

    // ── Delete ──
    function ondelete({ nodes: delNodes, edges: delEdges }: any) {
        pushUndo();
        if (delNodes?.length) {
            const realDels = delNodes.filter((n: any) => !n.data?.isVirtual);
            if (realDels.length > 0) {
                const deleteIds = new Set(realDels.map((n: any) => n.id));
                nodes = nodes.filter((n) => !deleteIds.has(n.id));
                edges = edges.filter(
                    (e) =>
                        !deleteIds.has(e.source) && !deleteIds.has(e.target),
                );
                if (selectedNode && deleteIds.has(selectedNode.id)) {
                    selectedNode = null;
                    showInspector = false;
                }
                isDirty = true;
            }
        }
        if (delEdges?.length) {
            const deleteIds = new Set(delEdges.map((e: any) => e.id));
            edges = edges.filter((e) => !deleteIds.has(e.id));
            isDirty = true;
        }
    }

    // ── Node click ──
    function onnodeclick({ node: clickedNode }: any) {
        selectedNode = clickedNode;
        showInspector = true;
    }

    function onpaneclick() {
        selectedNode = null;
        showInspector = false;
    }

    // ── Drop from palette / Dialog ──
    let createPosition: { x: number; y: number } | null = $state(null);

    function handlePaletteSelect(paletteData: any, pos?: { x: number; y: number } | null) {
        try {
            pushUndo();
            const existingIds = new Set(nodes.map((n) => n.id));
            
            // Determine position
            let nodePos = { x: 100, y: 100 };
            if (pos) {
                // If opening from context menu, pos is screen coordinates
                const container = document.querySelector('.graph-container')?.getBoundingClientRect();
                if (container) {
                    nodePos = {
                        x: pos.x - container.left,
                        y: pos.y - container.top,
                    };
                }
            }
            
            const newNode = createNodeFromPalette(
                paletteData,
                nodePos,
                existingIds,
            );
            nodes = [...nodes, newNode];
            isDirty = true;
        } catch {
            /* ignore */
        }
    }

    function handleDrop(e: DragEvent) {
        e.preventDefault();
        const raw = e.dataTransfer?.getData('application/dm-node');
        if (!raw) return;
        try {
            pushUndo();
            const paletteData = JSON.parse(raw);
            const container = (
                e.currentTarget as HTMLElement
            ).getBoundingClientRect();
            const position = {
                x: e.clientX - container.left,
                y: e.clientY - container.top,
            };
            const existingIds = new Set(nodes.map((n) => n.id));
            const newNode = createNodeFromPalette(
                paletteData,
                position,
                existingIds,
            );
            nodes = [...nodes, newNode];
            isDirty = true;
        } catch {
            /* ignore */
        }
    }

    function handleDragOver(e: DragEvent) {
        e.preventDefault();
        if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    }

    // ── Save ──
    async function saveAll() {
        isSaving = true;
        try {
            const newYaml = graphToYaml(nodes, edges, lastYaml);
            await post(`/dataflows/${dataflowName}`, { yaml: newYaml });
            const view = buildViewJson(nodes);
            await post(`/dataflows/${dataflowName}/view`, view);
            lastYaml = newYaml;
            isDirty = false;
            toast.success('Saved');
        } catch (e: any) {
            toast.error(`Save failed: ${e.message}`);
        } finally {
            isSaving = false;
        }
    }

    // ── Keyboard shortcuts ──
    function onkeydown(e: KeyboardEvent) {
        const meta = e.metaKey || e.ctrlKey;
        if (meta && e.key === 's') {
            e.preventDefault();
            saveAll();
        } else if (meta && e.key === 'z' && !e.shiftKey) {
            e.preventDefault();
            undo();
        } else if (meta && e.key === 'z' && e.shiftKey) {
            e.preventDefault();
            redo();
        } else if (meta && e.key === 'd') {
            e.preventDefault();
            if (selectedNode) duplicateNode();
        } else if (e.key === 'Backspace' || e.key === 'Delete') {
            // SvelteFlow handles delete natively, but we can rely on ondelete callback
        }
    }
</script>

<svelte:window on:keydown={onkeydown} />

<div class="editor-page">
    <!-- Top bar -->
    <div class="editor-topbar">
        <div class="flex items-center gap-3">
            <Button
                variant="ghost"
                size="sm"
                href="/dataflows/{dataflowName}"
            >
                <ArrowLeft class="size-4 mr-1" />
                Back
            </Button>
            <div class="h-5 w-px bg-border"></div>
            <h1 class="text-sm font-semibold font-mono">{dataflowName}</h1>
            {#if isDirty}
                <span class="text-xs text-amber-500 font-medium">● unsaved</span>
            {/if}
        </div>

        <div class="flex items-center gap-1">
            <Button
                variant="outline"
                size="sm"
                class="mr-1"
                onclick={() => {
                    createPosition = null;
                    showPalette = true;
                }}
            >
                <Plus class="size-4 mr-1" />
                Add Node
            </Button>

            <Button
                variant="outline"
                size="sm"
                class="mr-1"
                disabled={nodes.length === 0}
                onclick={applyAutoLayout}
                title="Auto Layout"
            >
                <LayoutGrid class="size-4 mr-1" />
                Auto Layout
            </Button>

            <Button variant="ghost" size="icon" class="h-8 w-8" onclick={toggleMode} title="Toggle Theme">
                {#if mode.current === "dark"}
                    <Sun class="size-4" />
                {:else}
                    <Moon class="size-4" />
                {/if}
            </Button>

            <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                        <Button variant="ghost" size="icon" class="h-8 w-8" {...props} title={$t("language")}>
                            <Languages class="size-4" />
                        </Button>
                    {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content side="top" align="start">
                    {#each ["en", "zh-CN"] as tag}
                        <DropdownMenu.Item>
                            <button
                                onclick={() => ($locale = tag)}
                                class="w-full h-full text-left flex items-center text-xs"
                            >
                                {tag === "en" ? $t("english") : $t("chinese")}
                            </button>
                        </DropdownMenu.Item>
                    {/each}
                </DropdownMenu.Content>
            </DropdownMenu.Root>

            <div class="h-5 w-px bg-border mx-1"></div>

            <Button
                variant="ghost"
                size="sm"
                class="h-8 w-8 px-0"
                disabled={undoStack.length === 0}
                onclick={undo}
                title="Undo (⌘Z)"
            >
                <Undo2 class="size-4" />
            </Button>
            <Button
                variant="ghost"
                size="sm"
                disabled={redoStack.length === 0}
                onclick={redo}
                title="Redo (⌘⇧Z)"
            >
                <Redo2 class="size-4" />
            </Button>

            <div class="h-5 w-px bg-border mx-1"></div>

            <Button
                variant={isDirty ? 'default' : 'outline'}
                size="sm"
                disabled={isSaving}
                onclick={saveAll}
            >
                <Save class="mr-2 size-4" />
                {isSaving ? 'Saving...' : isDirty ? 'Save' : 'Saved'}
            </Button>
        </div>
    </div>

    <!-- Main content: 3-panel layout -->
    {#if loading}
        <div class="flex-1 flex items-center justify-center text-muted-foreground">
            Loading...
        </div>
    {:else}
        <div class="editor-body">
            <NodePalette 
                bind:open={showPalette} 
                {createPosition} 
                onSelect={handlePaletteSelect} 
            />

            <div
                class="flex-1 min-h-0 min-w-0 relative graph-container"
                ondrop={handleDrop}
                ondragover={handleDragOver}
                role="application"
            >
                <SvelteFlow
                    bind:nodes
                    bind:edges
                    {nodeTypes}
                    {edgeTypes}
                    fitView
                    colorMode={colorMode}
                    proOptions={{ hideAttribution: true }}
                    {isValidConnection}
                    {onconnect}
                    {ondelete}
                    {onnodeclick}
                    {onpaneclick}
                    {onpanecontextmenu}
                    {onnodecontextmenu}
                    {onedgecontextmenu}
                >
                    <Controls />
                    <MiniMap position="bottom-left" />
                    <Background variant={BackgroundVariant.Dots} />
                </SvelteFlow>
            </div>

            {#if showInspector}
                <NodeInspector
                    node={selectedNode}
                    {edges}
                    dataflowName={dataflow.name}
                    onRenameNode={handleRenameNode}
                    onUpdateConfig={handleUpdateConfig}
                    onclose={() => (showInspector = false)}
                />
            {/if}

            <ContextMenu
                {...contextMenu}
                onAction={handleContextMenuAction}
                onClose={() => (contextMenu.visible = false)}
            />
        </div>
    {/if}
</div>

<style>
    .editor-page {
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 100vw;
        overflow: hidden;
    }

    .editor-topbar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 16px;
        border-bottom: 1px solid hsl(var(--border));
        background: hsl(var(--card));
        flex-shrink: 0;
        z-index: 10;
    }

    .editor-body {
        display: flex;
        flex: 1;
        min-height: 0;
        overflow: hidden;
    }

    .graph-container {
        min-height: 0;
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
    .graph-container :global(.svelte-flow__edge.selected .svelte-flow__edge-path) {
        stroke: #3b82f6;
        stroke-width: 2.5;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__edge-path) {
        stroke: #64748b;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__edge.selected .svelte-flow__edge-path) {
        stroke: #60a5fa;
    }

    /* ── Handle styling ── */
    .graph-container :global(.svelte-flow__handle) {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        border: 2px solid #cbd5e1;
        background: #ffffff;
    }
    .graph-container :global(.svelte-flow__handle:hover) {
        border-color: #3b82f6;
    }
    .graph-container :global(.svelte-flow__handle-connecting) {
        background: #22c55e;
        border-color: #22c55e;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__handle) {
        border-color: #475569;
        background: #1e293b;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__handle:hover) {
        border-color: #60a5fa;
    }

    /* ── Node selection ── */
    .graph-container :global(.svelte-flow__node.selected) {
        box-shadow: 0 0 0 2px #3b82f6;
        border-radius: 10px;
    }
    .graph-container :global(.svelte-flow.dark .svelte-flow__node.selected) {
        box-shadow: 0 0 0 2px #60a5fa;
    }

    /* ── Controls (dark) ── */
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

    /* ── Connection line ── */
    .graph-container :global(.svelte-flow__connection-path) {
        stroke: #3b82f6;
        stroke-width: 2;
    }
</style>
