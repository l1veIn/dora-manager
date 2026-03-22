<script lang="ts">
    import { get } from '$lib/api';
    import { onMount } from 'svelte';
    import { Search, Plus, GripVertical } from 'lucide-svelte';
    import { Input } from '$lib/components/ui/input/index.js';
    import { Badge } from '$lib/components/ui/badge/index.js';

    import * as Dialog from '$lib/components/ui/dialog/index.js';

    let {
        open = $bindable(false),
        createPosition,
        onSelect,
    }: {
        open?: boolean;
        createPosition?: { x: number; y: number } | null;
        onSelect?: (paletteData: any, position?: { x: number; y: number } | null) => void;
    } = $props();

    let allNodes = $state<any[]>([]);
    let searchQuery = $state('');
    let selectedCategory = $state<string | null>(null);
    let loading = $state(true);

    onMount(async () => {
        try {
            const data: any = await get('/nodes');
            allNodes = data.nodes || data || [];
        } catch {
            allNodes = [];
        } finally {
            loading = false;
        }
    });

    let categories = $derived(() => {
        const cats = new Set<string>();
        for (const n of allNodes) {
            const cat = n.display?.category || 'Other';
            cats.add(cat);
        }
        return [...cats].sort();
    });

    let filteredNodes = $derived(() => {
        let list = allNodes;
        if (selectedCategory) {
            list = list.filter(
                (n) => (n.display?.category || 'Other') === selectedCategory,
            );
        }
        if (searchQuery.trim()) {
            const q = searchQuery.toLowerCase();
            list = list.filter(
                (n) =>
                    n.id?.toLowerCase().includes(q) ||
                    n.name?.toLowerCase().includes(q) ||
                    n.display?.tags?.some((t: string) =>
                        t.toLowerCase().includes(q),
                    ),
            );
        }
        return list;
    });

    function handleDragStart(e: DragEvent, node: any) {
        e.dataTransfer?.setData(
            'application/dm-node',
            JSON.stringify(getPaletteData(node))
        );
        e.dataTransfer!.effectAllowed = 'move';
        // Hide dialog when dragging out, optional: open = false
    }

    function getPaletteData(node: any) {
        return {
            nodeId: node.id,
            nodeName: node.name,
            inputs: (node.ports || [])
                .filter((p: any) => p.direction === 'input')
                .map((p: any) => p.id),
            outputs: (node.ports || [])
                .filter((p: any) => p.direction === 'output')
                .map((p: any) => p.id),
            dynamicPorts: node.dynamic_ports || false,
        };
    }

    function handleSelect(node: any) {
        open = false;
        onSelect?.(getPaletteData(node), createPosition);
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[500px] h-[75vh] p-0 flex flex-col overflow-hidden">
        <Dialog.Header class="px-5 pt-5 pb-3 border-b shrink-0">
            <Dialog.Title>Add Node</Dialog.Title>
        </Dialog.Header>

        <!-- Search area -->
        <div class="px-5 py-3 border-b bg-muted/10 shrink-0">
            <div class="relative">
                <Search
                    class="absolute left-3 top-2.5 size-4 text-muted-foreground"
                />
                <Input
                    type="text"
                    placeholder="Search nodes by name, ID, or tags..."
                    class="pl-9 h-9"
                    bind:value={searchQuery}
                    autofocus
                />
            </div>

            <!-- Category filter chips -->
            <div class="flex flex-wrap gap-1.5 mt-3">
                <button
                    class="chip"
                    class:chip--active={!selectedCategory}
                    onclick={() => (selectedCategory = null)}
                >
                    All
                </button>
                {#each categories() as cat}
                    <button
                        class="chip"
                        class:chip--active={selectedCategory === cat}
                        onclick={() =>
                            (selectedCategory =
                                selectedCategory === cat ? null : cat)}
                    >
                        {cat}
                    </button>
                {/each}
            </div>
        </div>

        <!-- Node list -->
        <div class="flex-1 overflow-y-auto p-3 bg-muted/5">
            {#if loading}
                <div class="flex justify-center p-8 text-muted-foreground">Loading...</div>
            {:else if filteredNodes().length === 0}
                <div class="flex justify-center p-8 text-muted-foreground">No nodes found</div>
            {:else}
                <div class="grid grid-cols-2 gap-2">
                    {#each filteredNodes() as node (node.id)}
                        <button
                            class="palette-item"
                            draggable="true"
                            ondragstart={(e) => handleDragStart(e, node)}
                            onclick={() => handleSelect(node)}
                        >
                            <div class="flex-1 min-w-0 text-left">
                                <div class="text-sm font-medium truncate">
                                    {node.name || node.id}
                                </div>
                                <div class="text-[10px] text-muted-foreground truncate opacity-80 mt-0.5 font-mono">
                                    {node.id}
                                </div>
                            </div>
                            {#if node.display?.category}
                                <Badge variant="secondary" class="text-[9px] h-4 leading-none px-1.5 ml-2 shrink-0">
                                    {node.display.category}
                                </Badge>
                            {/if}
                        </button>
                    {/each}
                </div>
            {/if}
        </div>
    </Dialog.Content>
</Dialog.Root>

<style>
    .chip {
        font-size: 11px;
        padding: 3px 10px;
        border-radius: 12px;
        border: 1px solid hsl(var(--border));
        background: transparent;
        color: hsl(var(--muted-foreground));
        cursor: pointer;
        transition: all 0.15s;
    }
    .chip:hover {
        background: hsl(var(--accent));
    }
    .chip--active {
        background: hsl(var(--primary));
        color: hsl(var(--primary-foreground));
        border-color: hsl(var(--primary));
    }

    .palette-item {
        display: flex;
        align-items: center;
        padding: 10px 12px;
        border-radius: 8px;
        border: 1px solid transparent;
        cursor: pointer;
        background: hsl(var(--card));
        transition: all 0.15s;
        box-shadow: 0 1px 2px rgba(0,0,0,0.05);
    }
    .palette-item:hover {
        background: hsl(var(--accent));
        border-color: hsl(var(--border));
        transform: translateY(-1px);
        box-shadow: 0 4px 6px rgba(0,0,0,0.05);
    }
</style>
