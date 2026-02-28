<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { RefreshCw, Trash2, Download, Play, BookOpen } from "lucide-svelte";

    // 接收完整的节点数据
    let { 
        node, 
        isRegistry = false, 
        isInstalled = false, 
        operation = null, 
        onAction,
        onViewDetails
    } = $props<{
        node: any;
        isRegistry?: boolean;
        isInstalled?: boolean;
        operation?: string | null;
        onAction: (action: string, id: string) => void;
        onViewDetails?: (node: any) => void;
    }>();

    // 核心状态判断：是否已下载但未安装（缺少 executable）
    let needsInstall = $derived(isInstalled && (!node.executable || node.executable.trim() === ""));
</script>

<div
    class="border rounded-lg p-5 flex flex-col bg-card hover:border-slate-300 dark:hover:border-slate-700 transition-colors duration-200 {isRegistry && isInstalled && !needsInstall ? 'opacity-70' : ''}"
>
    <!-- Header -->
    <div class="flex items-start justify-between">
        <div class="font-bold font-mono text-base truncate pr-2 flex items-center gap-2">
            <!-- Details Trigger (Clickable Name) -->
            {#if !isRegistry || isInstalled}
                <button 
                    class="hover:underline hover:text-primary text-left truncate" 
                    onclick={() => onViewDetails && onViewDetails(node)}
                >
                    {node.name || node.id}
                </button>
            {:else}
                <span class="truncate">{node.name || node.id}</span>
            {/if}
        </div>
        
        {#if isRegistry}
            <div class="text-xs font-mono text-muted-foreground flex-shrink-0">
                ★ {node.stars || 0}
            </div>
        {:else}
            <Badge variant="outline" class="font-mono text-[10px] whitespace-nowrap flex-shrink-0">
                {node.version || "v0.0.0"}
            </Badge>
        {/if}
    </div>

    <!-- Description -->
    <div class="text-sm text-muted-foreground mt-2 line-clamp-2 h-10">
        {node.description || "No description provided."}
    </div>

    <Separator class="my-4" />

    <!-- Footer Actions -->
    <div class="mt-auto flex items-center justify-between">
        <div class="flex items-center gap-2">
            <Badge variant="secondary" class="text-[10px]">
                {node.language || "Unknown"}
            </Badge>
            {#if needsInstall}
                <Badge variant="destructive" class="text-[10px]">Not Installed</Badge>
            {/if}
        </div>

        <div class="flex items-center gap-2">
            <!-- Details Button -->
            {#if (!isRegistry || isInstalled) && onViewDetails}
                <Button 
                    variant="ghost" 
                    size="sm" 
                    class="h-8 px-2"
                    onclick={() => onViewDetails(node)}
                >
                    <BookOpen class="size-4" />
                </Button>
            {/if}

            {#if isRegistry}
                <!-- Remote Node Actions -->
                {#if isInstalled && !needsInstall}
                    <Button variant="outline" size="sm" class="h-8 px-3" disabled>
                        Installed ✓
                    </Button>
                {:else if needsInstall}
                    <Button
                        variant="default"
                        size="sm"
                        class="h-8 px-3"
                        disabled={operation === "installing"}
                        onclick={() => onAction("install", node.id)}
                    >
                        {#if operation === "installing"}
                            <RefreshCw class="size-3 animate-spin mr-2" /> Installing...
                        {:else}
                            <Play class="size-3 mr-2" /> Complete Install
                        {/if}
                    </Button>
                {:else}
                    <Button
                        variant="default"
                        size="sm"
                        class="h-8 px-3"
                        disabled={operation === "downloading"}
                        onclick={() => onAction("download", node.id)}
                    >
                        {#if operation === "downloading"}
                            <RefreshCw class="size-3 animate-spin mr-2" /> Downloading...
                        {:else}
                            <Download class="size-3 mr-2" /> Download
                        {/if}
                    </Button>
                {/if}
            {:else}
                <!-- Local Node Actions -->
                {#if needsInstall}
                    <Button
                        variant="default"
                        size="sm"
                        class="h-8 px-3"
                        disabled={operation === "installing"}
                        onclick={() => onAction("install", node.id)}
                    >
                        {#if operation === "installing"}
                            <RefreshCw class="size-3 animate-spin mr-2" /> Installing...
                        {:else}
                            <Play class="size-3 mr-2" /> Install
                        {/if}
                    </Button>
                {:else}
                    <Button
                        variant="outline"
                        size="sm"
                        class="h-8 px-3"
                        disabled={operation === "installing"}
                        onclick={() => onAction("install", node.id)}
                        title="Re-install this node (e.g. after code changes)"
                    >
                        {#if operation === "installing"}
                            <RefreshCw class="size-3 animate-spin mr-2" /> Re-installing...
                        {:else}
                            <RefreshCw class="size-3 mr-2" /> Re-install
                        {/if}
                    </Button>
                {/if}
                
                <Button
                    variant="ghost"
                    size="sm"
                    class="text-red-500 hover:text-red-600 hover:bg-red-500/10 h-8 px-2"
                    disabled={operation === "uninstalling"}
                    onclick={() => onAction("uninstall", node.id)}
                >
                    {#if operation === "uninstalling"}
                        <RefreshCw class="size-4 animate-spin mr-1" />
                    {:else}
                        <Trash2 class="size-4" />
                    {/if}
                </Button>
            {/if}
        </div>
    </div>
</div>
