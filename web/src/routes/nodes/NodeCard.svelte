<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { RefreshCw, Download, Trash2, UserRound } from "lucide-svelte";
    import {
        isInstalledNode,
        nodeAvatarSrc,
        nodeCategory,
        nodeOriginLabel,
        nodePrimaryMaintainer,
        nodeRuntimeLabel,
    } from "$lib/nodes/catalog";

    let {
        node,
        operation = null,
        onAction,
        href,
    } = $props<{
        node: any;
        operation?: string | null;
        onAction?: (action: string, id: string) => void;
        href?: string;
    }>();

    import { DropdownMenu } from "bits-ui";
    import { MoreVertical } from "lucide-svelte";
    import * as DropdownUI from "$lib/components/ui/dropdown-menu/index.js";

    let needsInstall = $derived(!isInstalledNode(node));
    let avatarBroken = $state(false);
    let avatarSrc = $derived(avatarBroken ? null : nodeAvatarSrc(node));
</script>

<svelte:element
    this={href ? "a" : "div"}
    {href}
    role={href ? undefined : "button"}
    tabindex={href ? undefined : 0}
    class="w-full text-left border rounded-lg p-5 flex flex-col bg-card hover:border-slate-400 dark:hover:border-slate-500 transition-colors duration-200 {operation
        ? 'opacity-50 pointer-events-none'
        : ''} focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 {href
        ? ''
        : 'cursor-pointer'} block hover:no-underline text-foreground"
>
    <!-- Header -->
    <div class="flex items-start justify-between w-full gap-3">
        <div class="flex items-start gap-3 min-w-0 flex-1">
            <div
                class="h-12 w-12 rounded-lg border bg-muted/40 overflow-hidden flex items-center justify-center shrink-0"
            >
                {#if avatarSrc}
                    <img
                        src={avatarSrc}
                        alt={`${node.name || node.id} avatar`}
                        class="h-full w-full object-cover"
                        onerror={() => (avatarBroken = true)}
                    />
                {:else}
                    <UserRound class="size-5 text-primary" />
                {/if}
            </div>
            <div class="pr-2 min-w-0 space-y-1 flex-1">
                <div
                    class="font-bold font-mono text-base truncate flex items-center gap-2"
                >
                    <span class="truncate">{node.name || node.id}</span>
                </div>
                <div class="flex items-center gap-2 flex-wrap">
                    <Badge
                        variant="outline"
                        class="font-mono text-[10px] whitespace-nowrap"
                    >
                        {nodeOriginLabel(node)}
                    </Badge>
                    <Badge variant="secondary" class="text-[10px]">
                        {nodeCategory(node)}
                    </Badge>
                    {#if needsInstall}
                        <Badge variant="destructive" class="text-[10px]"
                            >Not Installed</Badge
                        >
                    {:else}
                        <Badge
                            variant="outline"
                            class="text-[10px] bg-green-50 text-green-700 border-green-200"
                        >
                            Installed
                        </Badge>
                    {/if}
                </div>
            </div>
        </div>

        <div class="flex items-center gap-2">
            <Badge
                variant="outline"
                class="font-mono text-[10px] whitespace-nowrap flex-shrink-0"
            >
                {node.version || "v0.0.0"}
            </Badge>

            {#if onAction}
                <DropdownUI.Root>
                    <DropdownUI.Trigger
                        class="h-6 w-6 -mr-2 text-muted-foreground hover:text-foreground inline-flex items-center justify-center rounded-md text-sm font-medium whitespace-nowrap transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent"
                    >
                        <span class="sr-only">Open menu</span>
                        <MoreVertical class="h-4 w-4" />
                    </DropdownUI.Trigger>
                    <DropdownUI.Content align="end">
                        {#if needsInstall}
                            <DropdownUI.Item
                                onclick={(e) => {
                                    e.stopPropagation();
                                    e.preventDefault();
                                    onAction("install", node.id);
                                }}
                            >
                                <Download class="mr-2 h-4 w-4" />
                                <span>Install</span>
                            </DropdownUI.Item>
                        {:else}
                            <DropdownUI.Item
                                onclick={(e) => {
                                    e.stopPropagation();
                                    e.preventDefault();
                                    onAction("install", node.id);
                                }}
                            >
                                <RefreshCw class="mr-2 h-4 w-4" />
                                <span>Re-install</span>
                            </DropdownUI.Item>
                        {/if}
                        <DropdownUI.Separator />
                        <DropdownUI.Item
                            class="text-destructive focus:text-destructive"
                            onclick={(e) => {
                                e.stopPropagation();
                                e.preventDefault();
                                onAction("uninstall", node.id);
                            }}
                        >
                            <Trash2 class="mr-2 h-4 w-4" />
                            <span>Delete</span>
                        </DropdownUI.Item>
                    </DropdownUI.Content>
                </DropdownUI.Root>
            {/if}
        </div>
    </div>

    <!-- Description -->
    <div class="text-sm text-muted-foreground mt-2 line-clamp-2 min-h-10 w-full">
        {node.description || "No description provided."}
    </div>

    <Separator class="my-4" />

    <!-- Footer Status -->
    <div class="mt-auto space-y-3 w-full">
        <div class="flex items-center gap-2 flex-wrap">
            <Badge variant="secondary" class="text-[10px]">
                {nodeRuntimeLabel(node)}
            </Badge>
            {#if node.display?.tags?.length}
                {#each node.display.tags.slice(0, 2) as tag}
                    <Badge variant="outline" class="text-[10px]">{tag}</Badge>
                {/each}
            {/if}
        </div>

        <div class="flex items-center justify-between gap-2">
            <div class="text-xs text-muted-foreground flex items-center gap-1.5 min-w-0">
                <UserRound class="size-3 shrink-0" />
                <span class="truncate">{nodePrimaryMaintainer(node) || "Unknown maintainer"}</span>
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
</svelte:element>
