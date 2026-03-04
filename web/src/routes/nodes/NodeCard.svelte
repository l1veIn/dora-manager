<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { RefreshCw, Star, Download } from "lucide-svelte";

    let {
        node,
        operation = null,
        onAction,
        onViewDetails,
    } = $props<{
        node: any;
        operation?: string | null;
        onAction?: (action: string, id: string) => void;
        onViewDetails?: (node: any) => void;
    }>();

    let needsInstall = $derived(
        !node.executable || node.executable.trim() === "",
    );
</script>

<div
    role="button"
    tabindex="0"
    class="w-full text-left border rounded-lg p-5 flex flex-col bg-card hover:border-slate-400 dark:hover:border-slate-500 transition-colors duration-200 {operation
        ? 'opacity-50 pointer-events-none'
        : ''} focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 cursor-pointer"
    onclick={() => onViewDetails && onViewDetails(node)}
    onkeydown={(e) => e.key === "Enter" && onViewDetails && onViewDetails(node)}
>
    <!-- Header -->
    <div class="flex items-start justify-between w-full">
        <div
            class="font-bold font-mono text-base truncate pr-2 flex items-center gap-2"
        >
            <span class="truncate">{node.name || node.id}</span>
        </div>

        <Badge
            variant="outline"
            class="font-mono text-[10px] whitespace-nowrap flex-shrink-0"
        >
            {node.version || "v0.0.0"}
        </Badge>
    </div>

    <!-- Description -->
    <div class="text-sm text-muted-foreground mt-2 line-clamp-2 h-10 w-full">
        {node.description || "No description provided."}
    </div>

    <Separator class="my-4" />

    <!-- Footer Status -->
    <div class="mt-auto flex items-center justify-between w-full">
        <div class="flex items-center gap-2">
            <Badge variant="secondary" class="text-[10px]">
                {node.language || "Unknown"}
            </Badge>
            {#if needsInstall}
                <Badge variant="destructive" class="text-[10px]"
                    >Not Installed</Badge
                >
            {/if}
        </div>

        <div class="flex items-center gap-2">
            {#if operation === "downloading"}
                <span class="text-xs text-muted-foreground flex items-center"
                    ><RefreshCw class="size-3 animate-spin mr-1" /> Downloading...</span
                >
            {:else if operation === "installing"}
                <span class="text-xs text-muted-foreground flex items-center"
                    ><RefreshCw class="size-3 animate-spin mr-1" /> Installing...</span
                >
            {:else if operation === "uninstalling"}
                <span class="text-xs text-muted-foreground flex items-center"
                    ><RefreshCw class="size-3 animate-spin mr-1" /> Uninstalling...</span
                >
            {/if}
        </div>
    </div>
</div>
