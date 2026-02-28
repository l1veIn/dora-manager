<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { RefreshCw, Star, Download } from "lucide-svelte";

    // 接收完整的节点数据
    let {
        node,
        isRegistry = false,
        isInstalled = false,
        operation = null,
        onAction,
        onViewDetails,
    } = $props<{
        node: any;
        isRegistry?: boolean;
        isInstalled?: boolean;
        operation?: string | null;
        onAction?: (action: string, id: string) => void;
        onViewDetails?: (node: any) => void;
    }>();

    // 核心状态判断：是否已下载但未安装（缺少 executable）
    let needsInstall = $derived(
        isInstalled && (!node.executable || node.executable.trim() === ""),
    );
</script>

<div
    role="button"
    tabindex="0"
    class="w-full text-left border rounded-lg p-5 flex flex-col bg-card hover:border-slate-400 dark:hover:border-slate-500 transition-colors duration-200 {isRegistry &&
    isInstalled &&
    !needsInstall
        ? 'opacity-70'
        : ''} {operation
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

        {#if isRegistry}
            <div
                class="text-xs font-mono text-muted-foreground flex-shrink-0 flex items-center gap-1"
            >
                <Star class="size-3" />
                {node.stars || 0}
            </div>
        {:else}
            <Badge
                variant="outline"
                class="font-mono text-[10px] whitespace-nowrap flex-shrink-0"
            >
                {node.version || "v0.0.0"}
            </Badge>
        {/if}
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
            {:else if isRegistry && isInstalled}
                <Badge variant="outline" class="text-[10px]">Installed</Badge>
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
            {:else if isRegistry && !isInstalled}
                <Button
                    variant="default"
                    size="sm"
                    class="h-7 px-3 text-xs cursor-pointer z-10"
                    onclick={(e: Event) => {
                        e.stopPropagation();
                        if (onAction) onAction("download", node.id);
                    }}
                >
                    <Download class="size-3 mr-1" /> Download
                </Button>
            {/if}
        </div>
    </div>
</div>
