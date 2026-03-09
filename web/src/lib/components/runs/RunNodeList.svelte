<script lang="ts">
    import { FileText } from "lucide-svelte";
    import { Badge } from "$lib/components/ui/badge/index.js";

    let {
        nodes = [],
        metrics = null,
        selectedNodeId = $bindable(),
    } = $props<{
        nodes: any[];
        metrics?: any;
        selectedNodeId: string;
    }>();

    function formatSize(bytes: number) {
        if (!bytes) return "(empty)";
        if (bytes < 1024) return `${bytes} B`;
        return `${(bytes / 1024).toFixed(1)} KB`;
    }

    function getNodeMetrics(nodeId: string) {
        if (!metrics?.nodes) return null;
        return metrics.nodes.find((n: any) => n.id === nodeId) || null;
    }
</script>

<div class="flex flex-col flex-1 min-h-0 h-full w-full">
    <div class="px-4 border-b bg-muted/20 flex items-center shrink-0 h-11">
        <span
            class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider"
            >Node Logs</span
        >
    </div>
    <div class="p-2 overflow-y-auto flex-1 h-full min-h-0">
        {#if nodes.length === 0}
            <div class="py-8 text-center text-sm text-muted-foreground">
                No nodes found
            </div>
        {:else}
            <ul class="flex flex-col gap-0.5">
                {#each nodes as node}
                    {@const nm = getNodeMetrics(node.id)}
                    <li>
                        <button
                            class="w-full flex items-center justify-between px-3 py-2.5 rounded-md text-sm transition-colors text-left border {selectedNodeId ===
                            node.id
                                ? 'bg-primary/10 border-primary/20 text-primary font-medium'
                                : 'bg-transparent border-transparent hover:bg-muted/50 text-muted-foreground hover:text-foreground'}"
                            onclick={() => {
                                selectedNodeId = node.id;
                            }}
                        >
                            <div
                                class="flex items-center gap-2 overflow-hidden"
                            >
                                <FileText
                                    class="size-3.5 shrink-0 {selectedNodeId ===
                                    node.id
                                        ? 'text-primary'
                                        : 'text-muted-foreground/60'}"
                                />
                                <span class="truncate tracking-tight"
                                    >{node.id}</span
                                >
                            </div>
                            <div
                                class="flex items-center gap-1.5 ml-2 shrink-0"
                            >
                                {#if nm}
                                    <Badge
                                        variant="outline"
                                        class="text-[9px] font-mono px-1 py-0 rounded-sm font-normal {selectedNodeId ===
                                        node.id
                                            ? 'border-orange-300 text-orange-700 bg-orange-50 dark:border-orange-800/60 dark:text-orange-300 dark:bg-orange-950/30'
                                            : 'border-orange-200/50 text-orange-600/70 bg-transparent dark:border-orange-900/30 dark:text-orange-500/70'}"
                                        title="CPU 占用"
                                    >
                                        {nm.cpu || "-"}
                                    </Badge>
                                    <Badge
                                        variant="outline"
                                        class="text-[9px] font-mono px-1 py-0 rounded-sm font-normal {selectedNodeId ===
                                        node.id
                                            ? 'border-blue-300 text-blue-700 bg-blue-50 dark:border-blue-800/60 dark:text-blue-300 dark:bg-blue-950/30'
                                            : 'border-blue-200/50 text-blue-600/70 bg-transparent dark:border-blue-900/30 dark:text-blue-500/70'}"
                                        title="Memory 占用"
                                    >
                                        {nm.memory || "-"}
                                    </Badge>
                                {:else}
                                    <span
                                        class="text-[10px] font-mono {selectedNodeId ===
                                        node.id
                                            ? 'text-primary/70'
                                            : 'text-muted-foreground/50'}"
                                    >
                                        {formatSize(node.log_size)}
                                    </span>
                                {/if}
                            </div>
                        </button>
                    </li>
                {/each}
            </ul>
        {/if}
    </div>
</div>
