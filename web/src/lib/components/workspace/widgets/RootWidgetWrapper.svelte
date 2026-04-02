<script lang="ts">
    import { X, Maximize2, Minimize2 } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    let { node, api, children } = $props<{ node: any; api: any; children: any }>();

    let isMaximized = $state(false);

    function toggleMaximize() {
        isMaximized = !isMaximized;
    }
</script>

<svelte:window on:keydown={(e) => { if (e.key === 'Escape' && isMaximized) isMaximized = false; }} />

<!-- Maximize Overlay Trap -->
<div class={isMaximized ? "fixed inset-0 z-50 p-6 bg-background/80 backdrop-blur-sm" : "w-full h-full"}>
    <div 
        class="flex flex-col w-full h-full bg-background relative overflow-hidden border rounded-md shadow-sm transition-transform {isMaximized ? 'shadow-2xl rounded-lg scale-[1.01]' : ''}"
        role="presentation"
        ondblclick={toggleMaximize}
    >
        <!-- Drag Handle (Title Bar) -->
        <div class="grid-drag-handle h-8 flex shrink-0 items-center justify-between px-2 border-b bg-muted/40 cursor-grab active:cursor-grabbing hover:bg-muted/60 transition-colors" title="Double click to maximize">
            <div class="text-xs font-medium truncate flex-1 flex gap-2 items-center text-muted-foreground pointer-events-none">
                {#if node.widgetType === "stream"}
                    <div class="w-2 h-2 rounded-full bg-blue-500"></div> Stream
                {:else if node.widgetType === "input"}
                    <div class="w-2 h-2 rounded-full bg-orange-500"></div> Input
                {:else if node.widgetType === "terminal"}
                    <div class="w-2 h-2 rounded-full bg-zinc-800 dark:bg-zinc-200"></div> Terminal
                {:else}
                    <div class="w-2 h-2 rounded-full bg-muted-foreground"></div> Widget
                {/if}
            </div>
            <div class="flex items-center gap-0">
                <Button variant="ghost" size="icon" class="h-6 w-6" title={isMaximized ? "Restore" : "Maximize"} onclick={(e) => { e.stopPropagation(); toggleMaximize(); }}>
                    {#if isMaximized}
                        <Minimize2 class="h-3 w-3" />
                    {:else}
                        <Maximize2 class="h-3 w-3" />
                    {/if}
                </Button>
                <Button variant="ghost" size="icon" class="h-6 w-6 hover:bg-destructive/10 hover:text-destructive" title="Close Panel" onclick={(e) => { e.stopPropagation(); api.close(node.id); }}>
                    <X class="h-3.5 w-3.5" />
                </Button>
            </div>
        </div>
        <div class="flex-1 min-h-0 relative">
            {@render children()}
        </div>
    </div>
</div>
