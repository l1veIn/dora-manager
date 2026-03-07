<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { RefreshCw, Star, Download, Trash2 } from "lucide-svelte";

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

    let needsInstall = $derived(
        !node.executable || node.executable.trim() === "",
    );
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
    <div class="flex items-start justify-between w-full">
        <div
            class="font-bold font-mono text-base truncate pr-2 flex items-center gap-2"
        >
            <span class="truncate">{node.name || node.id}</span>
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
</svelte:element>
