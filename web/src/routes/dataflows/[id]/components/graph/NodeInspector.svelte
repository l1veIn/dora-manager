<script lang="ts">
    import { Badge } from '$lib/components/ui/badge/index.js';
    import { Input } from '$lib/components/ui/input/index.js';
    import { X, GripHorizontal } from 'lucide-svelte';
    import type { DmFlowNode, DmFlowEdge } from './types';
    import InspectorConfig from './InspectorConfig.svelte';
    import { onMount } from 'svelte';

    let {
        node,
        edges = [],
        dataflowName = '',
        onclose,
        onRenameNode,
        onUpdateConfig,
    }: {
        node: DmFlowNode | null;
        edges?: DmFlowEdge[];
        dataflowName?: string;
        onclose?: () => void;
        onRenameNode?: (oldId: string, newId: string) => void;
        onUpdateConfig?: (newConfig: any) => void;
    } = $props();

    const PANEL_MARGIN = 20;
    const MIN_PANEL_WIDTH = 360;
    const MAX_PANEL_WIDTH = 560;
    const MIN_PANEL_HEIGHT = 320;
    const MAX_PANEL_HEIGHT = 760;

    // Bounds and dragging
    let bounds = $state({ x: PANEL_MARGIN, y: 80, w: 420, h: 620 });
    let isDragging = $state(false);
    let dragStart = { x: 0, y: 0, bx: 0, by: 0 };
    
    // Resizing
    let isResizing = $state(false);
    let resizeStart = { x: 0, y: 0, bw: 0, bh: 0 };

    function clampBounds(next: { x: number; y: number; w: number; h: number }, viewportWidth: number, viewportHeight: number) {
        const maxWidth = Math.max(MIN_PANEL_WIDTH, viewportWidth - PANEL_MARGIN * 2);
        const maxHeight = Math.max(MIN_PANEL_HEIGHT, viewportHeight - PANEL_MARGIN * 2);
        const width = Math.min(Math.max(next.w, MIN_PANEL_WIDTH), Math.min(MAX_PANEL_WIDTH, maxWidth));
        const height = Math.min(Math.max(next.h, MIN_PANEL_HEIGHT), Math.min(MAX_PANEL_HEIGHT, maxHeight));
        const x = Math.min(
            Math.max(next.x, PANEL_MARGIN),
            Math.max(PANEL_MARGIN, viewportWidth - width - PANEL_MARGIN),
        );
        const y = Math.min(
            Math.max(next.y, PANEL_MARGIN),
            Math.max(PANEL_MARGIN, viewportHeight - height - PANEL_MARGIN),
        );

        return { x, y, w: width, h: height };
    }

    function defaultBounds(viewportWidth: number, viewportHeight: number) {
        const w = Math.min(
            Math.max(420, Math.round(viewportWidth * 0.32)),
            MAX_PANEL_WIDTH,
            Math.max(MIN_PANEL_WIDTH, viewportWidth - PANEL_MARGIN * 2),
        );
        const h = Math.min(
            Math.max(620, Math.round(viewportHeight * 0.72)),
            MAX_PANEL_HEIGHT,
            Math.max(MIN_PANEL_HEIGHT, viewportHeight - PANEL_MARGIN * 2),
        );

        return {
            x: Math.max(PANEL_MARGIN, viewportWidth - w - PANEL_MARGIN),
            y: Math.max(PANEL_MARGIN, Math.round((viewportHeight - h) / 2)),
            w,
            h,
        };
    }

    onMount(() => {
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;
        const cached = localStorage.getItem('dm-inspector-bounds');
        if (cached) {
            try {
                bounds = clampBounds(JSON.parse(cached), viewportWidth, viewportHeight);
            } catch {
                bounds = defaultBounds(viewportWidth, viewportHeight);
            }
        } else {
            bounds = defaultBounds(viewportWidth, viewportHeight);
        }

        const onWindowResize = () => {
            bounds = clampBounds(bounds, window.innerWidth, window.innerHeight);
        };

        window.addEventListener('resize', onWindowResize);
        return () => window.removeEventListener('resize', onWindowResize);
    });

    function saveBounds() {
        localStorage.setItem('dm-inspector-bounds', JSON.stringify(bounds));
    }

    // Drag handlers
    function onTitleMousedown(e: MouseEvent) {
        if ((e.target as HTMLElement).closest('button')) return;
        isDragging = true;
        dragStart = { x: e.clientX, y: e.clientY, bx: bounds.x, by: bounds.y };
        window.addEventListener('mousemove', onWindowMousemove);
        window.addEventListener('mouseup', onWindowMouseup);
    }
    
    // Resize handlers
    function onResizeMousedown(e: MouseEvent) {
        e.preventDefault();
        isResizing = true;
        resizeStart = { x: e.clientX, y: e.clientY, bw: bounds.w, bh: bounds.h };
        window.addEventListener('mousemove', onWindowMousemove);
        window.addEventListener('mouseup', onWindowMouseup);
    }

    function onWindowMousemove(e: MouseEvent) {
        if (isDragging) {
            bounds = clampBounds(
                {
                    ...bounds,
                    x: dragStart.bx + (e.clientX - dragStart.x),
                    y: dragStart.by + (e.clientY - dragStart.y),
                },
                window.innerWidth,
                window.innerHeight,
            );
        } else if (isResizing) {
            bounds = clampBounds(
                {
                    ...bounds,
                    w: resizeStart.bw + (e.clientX - resizeStart.x),
                    h: resizeStart.bh + (e.clientY - resizeStart.y),
                },
                window.innerWidth,
                window.innerHeight,
            );
        }
    }

    function onWindowMouseup() {
        if (isDragging || isResizing) saveBounds();
        isDragging = false;
        isResizing = false;
        window.removeEventListener('mousemove', onWindowMousemove);
        window.removeEventListener('mouseup', onWindowMouseup);
    }

    // Node ID editing
    let isEditingId = $state(false);
    let editIdValue = $state('');

    function startEditId() {
        if (!node) return;
        editIdValue = node.id;
        isEditingId = true;
    }

    function commitEditId() {
        if (!node || !isEditingId) return;
        const newId = editIdValue.trim();
        if (newId && newId !== node.id && onRenameNode) {
            onRenameNode(node.id, newId);
        }
        isEditingId = false;
    }

    function onIdKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter') commitEditId();
        else if (e.key === 'Escape') isEditingId = false;
    }

    // Tabs
    let activeTab = $state<'info'|'config'>('info');

    // Calculate connections
    function getConnections(port: string, type: 'in'|'out') {
        if (!node || !edges) return [];
        if (type === 'in') {
            return edges.filter(e => e.target === node.id && e.targetHandle === `in-${port}`).map(e => e.source);
        } else {
            return edges.filter(e => e.source === node.id && e.sourceHandle === `out-${port}`).map(e => e.target);
        }
    }
</script>

{#if node}
    <div 
        class="fixed inspector-window shadow-2xl border rounded-lg bg-card flex flex-col z-[100] text-card-foreground"
        style="left: {bounds.x}px; top: {bounds.y}px; width: {bounds.w}px; height: {bounds.h}px;"
    >
        <!-- Titlebar -->
        <div 
            class="flex items-center justify-between p-2 pl-3 border-b bg-muted/40 cursor-grab active:cursor-grabbing shrink-0"
            onmousedown={onTitleMousedown}
            role="presentation"
        >
            <div class="flex items-center gap-2 overflow-hidden flex-1 select-none pointer-events-none">
                <GripHorizontal class="size-4 text-muted-foreground shrink-0 opacity-60" />
                <h3 class="text-sm font-semibold truncate">Inspector</h3>
            </div>
            <!-- svelte-ignore a11y_consider_explicit_label -->
            <button class="h-7 w-7 rounded-md hover:bg-black/5 dark:hover:bg-white/10 flex items-center justify-center shrink-0 transition-colors" onclick={onclose}>
                <X class="size-4" />
            </button>
        </div>

        <!-- content -->
        <div class="flex-1 flex flex-col min-h-0 bg-card">
            <!-- Tabs header -->
            <div class="flex border-b shrink-0 px-2 pt-1 gap-2 bg-muted/10">
                <button 
                    class="px-3 py-1.5 text-xs font-medium border-b-2 transition-colors {activeTab === 'info' ? 'border-primary text-foreground' : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/20'}"
                    onclick={() => activeTab = 'info'}
                >
                    Info & Ports
                </button>
                <button 
                    class="px-3 py-1.5 text-xs font-medium border-b-2 transition-colors {activeTab === 'config' ? 'border-primary text-foreground' : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/20'}"
                    onclick={() => activeTab = 'config'}
                >
                    Configuration
                </button>
            </div>

            <div class="flex-1 overflow-y-auto p-4">
                {#if activeTab === 'info'}
                    <div class="space-y-6">
                        <!-- ID section -->
                        <div>
                            <span class="text-[10px] text-muted-foreground uppercase tracking-wide font-semibold">Node ID</span>
                            {#if isEditingId}
                                <div class="mt-1.5 flex gap-2">
                                    <!-- svelte-ignore a11y_autofocus -->
                                    <Input 
                                        bind:value={editIdValue} 
                                        onblur={commitEditId}
                                        onkeydown={onIdKeydown}
                                        class="h-8 text-xs font-mono"
                                        autofocus
                                    />
                                </div>
                            {:else}
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                                <p 
                                    class="text-sm font-mono mt-1 hover:bg-muted/50 p-1.5 -ml-1.5 rounded cursor-text transition-colors border border-transparent hover:border-border"
                                    onclick={startEditId}
                                    title="Click to rename"
                                >
                                    {node.id}
                                </p>
                            {/if}
                        </div>

                        <!-- Type section -->
                        <div>
                            <span class="text-[10px] text-muted-foreground uppercase tracking-wide font-semibold">Type</span>
                            <div class="mt-1 flex items-center gap-2">
                                <p class="text-sm font-mono">{node.data.nodeType}</p>
                                {#if node.data.isVirtual}
                                    <Badge variant="secondary" class="text-[10px] h-5 py-0 px-1.5 font-normal">Virtual ({node.data.virtualKind})</Badge>
                                {/if}
                            </div>
                        </div>

                        <div class="h-px bg-border/50"></div>

                        <!-- Inputs -->
                        {#if (node.data.inputs as string[]).length > 0}
                            <div>
                                <span class="text-[10px] text-muted-foreground uppercase tracking-wide font-semibold flex mb-2">Input Ports</span>
                                <div class="space-y-2">
                                    {#each node.data.inputs as port}
                                        {@const conns = getConnections(port, 'in')}
                                        <div class="text-xs bg-muted/30 border rounded-md p-2.5">
                                            <div class="font-mono font-medium mb-1.5 flex justify-between items-center">
                                                <span>{port}</span>
                                                <span class="text-[9px] px-1.5 py-0.5 rounded-sm {conns.length > 0 ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'}">
                                                    {conns.length > 0 ? 'Connected' : 'Unconnected'}
                                                </span>
                                            </div>
                                            {#if conns.length > 0}
                                                <div class="text-[10px] text-muted-foreground flex flex-col gap-1 mt-2 p-1.5 bg-background border rounded">
                                                    {#each conns as c}
                                                        <span class="truncate" title={c}>← {c}</span>
                                                    {/each}
                                                </div>
                                            {/if}
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                        <!-- Outputs -->
                        {#if (node.data.outputs as string[]).length > 0}
                            <div>
                                <span class="text-[10px] text-muted-foreground uppercase tracking-wide font-semibold flex mb-2">Output Ports</span>
                                <div class="space-y-2">
                                    {#each node.data.outputs as port}
                                        {@const conns = getConnections(port, 'out')}
                                        <div class="text-xs bg-muted/30 border rounded-md p-2.5">
                                            <div class="font-mono font-medium mb-1.5 flex justify-between items-center">
                                                <span>{port}</span>
                                                <span class="text-[9px] px-1.5 py-0.5 rounded-sm {conns.length > 0 ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'}">
                                                    {conns.length > 0 ? `${conns.length} Connected` : 'Unconnected'}
                                                </span>
                                            </div>
                                            {#if conns.length > 0}
                                                <div class="text-[10px] text-muted-foreground flex flex-col gap-1 mt-2 p-1.5 bg-background border rounded">
                                                    {#each conns as c}
                                                        <span class="truncate" title={c}>→ {c}</span>
                                                    {/each}
                                                </div>
                                            {/if}
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        {/if}
                    </div>
                {:else if activeTab === 'config'}
                    {#if onUpdateConfig}
                        <InspectorConfig {node} {onUpdateConfig} {dataflowName} />
                    {:else}
                        <div class="text-center p-4 text-xs text-muted-foreground mt-4 border border-dashed rounded bg-muted/10">
                            Config not available.
                        </div>
                    {/if}
                {/if}
            </div>
        </div>
        
        <!-- Resize Handle -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div 
            class="absolute bottom-0 right-0 w-4 h-4 cursor-se-resize z-10 before:content-[''] before:absolute before:bottom-1 before:right-1 before:w-2 before:h-2 before:border-b-2 before:border-r-2 before:border-muted-foreground before:opacity-30 hover:before:opacity-100"
            onmousedown={onResizeMousedown}
        ></div>
    </div>
{/if}

<style>
    .inspector-window {
        box-shadow: 0 10px 40px rgba(0,0,0,0.1), 0 0 0 1px inset rgba(255,255,255,0.05);
    }
</style>
