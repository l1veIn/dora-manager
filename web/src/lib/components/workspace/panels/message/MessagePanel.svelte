<script lang="ts">
    import { untrack } from "svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { ChevronDown, Loader2 } from "lucide-svelte";
    import type { PanelRendererProps } from "../types";
    import MessageItem from "./MessageItem.svelte";
    import { createMessageHistoryState, summarizeSelection } from "./message-state.svelte.js";

    const DEFAULT_TAGS = ["text", "image", "json", "markdown", "audio", "video"];

    let {
        item,
        context,
        onConfigChange,
    }: PanelRendererProps = $props();

    function ensureConfig() {
        if (!item.config) item.config = {};
        if (!Array.isArray(item.config.nodes) || item.config.nodes.length === 0) {
            item.config.nodes = ["*"];
        }
        if (!Array.isArray(item.config.tags) || item.config.tags.length === 0) {
            item.config.tags = ["*"];
        }
    }

    ensureConfig();

    let container = $state<HTMLElement | null>(null);
    let isUserScrolling = $state(false);

    let availableNodes = $derived(
        context.snapshots
            .map((snapshot: any) => snapshot.node_id)
            .filter((value: string, index: number, items: string[]) => value && items.indexOf(value) === index),
    );
    let availableTags = $derived(
        [...DEFAULT_TAGS, ...context.snapshots.map((snapshot: any) => snapshot.tag)]
            .filter((value: string, index: number, items: string[]) => value && items.indexOf(value) === index),
    );
    let selectedNodes = $derived(Array.isArray(item.config.nodes) ? item.config.nodes : ["*"]);
    let selectedTags = $derived(Array.isArray(item.config.tags) ? item.config.tags : ["*"]);
    const history = createMessageHistoryState(
        () => context.runId,
        () => ({ nodes: selectedNodes, tags: selectedTags }),
    );

    function applyFilters() {
        history.reset();
        onConfigChange?.();
        history.loadInitial(() => scrollToBottom(true));
    }

    function setAllNodes() {
        ensureConfig();
        item.config.nodes = ["*"];
        applyFilters();
    }

    function setAllTags() {
        ensureConfig();
        item.config.tags = ["*"];
        applyFilters();
    }

    function toggleNode(value: string) {
        ensureConfig();
        const current = selectedNodes.includes("*") ? [] : [...selectedNodes];
        const next = current.includes(value) ? current.filter((item) => item !== value) : [...current, value];
        item.config.nodes = next.length > 0 ? next : ["*"];
        applyFilters();
    }

    function toggleTag(value: string) {
        ensureConfig();
        const current = selectedTags.includes("*") ? [] : [...selectedTags];
        const next = current.includes(value) ? current.filter((item) => item !== value) : [...current, value];
        item.config.tags = next.length > 0 ? next : ["*"];
        applyFilters();
    }

    function handleMenuSelect(event: Event, action: () => void) {
        event.preventDefault();
        action();
    }

    function scrollToBottom(force = false) {
        if (container) {
            setTimeout(() => {
                if (container) container.scrollTop = container.scrollHeight;
            }, force ? 50 : 10);
        }
    }

    function handleScroll() {
        if (!container) return;
        const { scrollTop, scrollHeight, clientHeight } = container;
        isUserScrolling = Math.abs(scrollHeight - clientHeight - scrollTop) > 40;
        if (scrollTop < 10 && !history.fetchingOld && history.hasMoreOld) {
            history.loadOld((previousHeight) => {
                if (container) {
                    setTimeout(() => {
                        if (container) container.scrollTop = container.scrollHeight - previousHeight;
                    }, 0);
                }
            }, scrollHeight);
        }
    }

    $effect(() => {
        context.refreshToken;
        untrack(() => {
            if (history.messages.length > 0) {
                history.loadNew(() => {
                    if (!isUserScrolling) scrollToBottom();
                });
            } else {
                history.loadInitial(() => scrollToBottom(true));
            }
        });
    });
</script>

<div class="flex flex-col h-full w-full overflow-hidden bg-background">
    <div class="px-3 h-8 border-b bg-muted/20 flex items-center justify-between shrink-0">
        <div class="flex-1"></div>
        <div class="flex items-center gap-1.5">
            <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                        <Button {...props} variant="ghost" size="sm" class="h-7 w-auto max-w-[148px] justify-between gap-2 rounded-full border-0 bg-muted/20 px-2.5 text-[11px] font-mono text-foreground/90 shadow-none hover:bg-muted/35">
                            <span class="min-w-0 truncate">{summarizeSelection(selectedNodes, "All Nodes")}</span>
                            <ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
                        </Button>
                    {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end" class="w-56">
                    <DropdownMenu.Label>Filter Nodes</DropdownMenu.Label>
                    <DropdownMenu.Separator />
                    <DropdownMenu.CheckboxItem checked={selectedNodes.includes("*")} onclick={(event) => handleMenuSelect(event, setAllNodes)}>
                        All Nodes
                    </DropdownMenu.CheckboxItem>
                    <DropdownMenu.Separator />
                    {#each availableNodes as nodeId}
                        <DropdownMenu.CheckboxItem checked={!selectedNodes.includes("*") && selectedNodes.includes(nodeId)} onclick={(event) => handleMenuSelect(event, () => toggleNode(nodeId))}>
                            {nodeId}
                        </DropdownMenu.CheckboxItem>
                    {/each}
                </DropdownMenu.Content>
            </DropdownMenu.Root>

            <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                        <Button {...props} variant="ghost" size="sm" class="h-7 w-auto max-w-[148px] justify-between gap-2 rounded-full border-0 bg-muted/20 px-2.5 text-[11px] font-mono text-foreground/90 shadow-none hover:bg-muted/35">
                            <span class="min-w-0 truncate">{summarizeSelection(selectedTags, "All Tags")}</span>
                            <ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
                        </Button>
                    {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end" class="w-56">
                    <DropdownMenu.Label>Filter Tags</DropdownMenu.Label>
                    <DropdownMenu.Separator />
                    <DropdownMenu.CheckboxItem checked={selectedTags.includes("*")} onclick={(event) => handleMenuSelect(event, setAllTags)}>
                        All Tags
                    </DropdownMenu.CheckboxItem>
                    <DropdownMenu.Separator />
                    {#each availableTags as tag}
                        <DropdownMenu.CheckboxItem checked={!selectedTags.includes("*") && selectedTags.includes(tag)} onclick={(event) => handleMenuSelect(event, () => toggleTag(tag))}>
                            {tag}
                        </DropdownMenu.CheckboxItem>
                    {/each}
                </DropdownMenu.Content>
            </DropdownMenu.Root>
        </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4 space-y-4 bg-muted/10 relative" bind:this={container} onscroll={handleScroll}>
        {#if history.fetchingOld}
            <div class="flex items-center justify-center p-2 text-muted-foreground/60 w-full mb-4">
                <Loader2 class="size-4 animate-spin" />
            </div>
        {:else if !history.hasMoreOld && history.messages.length > 0}
            <div class="flex items-center justify-center py-2 text-[10px] text-muted-foreground/40 uppercase tracking-widest w-full mb-4">
                -- Top of History --
            </div>
        {/if}

        {#if history.messages.length === 0 && !history.fetching}
            <div class="absolute inset-0 flex flex-col items-center justify-center text-sm text-muted-foreground/60 tracking-wider">
                <span class="font-mono">&gt; No Messages</span>
            </div>
        {/if}

        <div class="flex flex-col gap-4 justify-end min-h-full">
            {#each history.messages as entry (entry.from + "_" + entry.seq)}
                <MessageItem runId={context.runId} {entry} />
            {/each}
        </div>

        {#if history.fetching && history.messages.length === 0}
            <div class="flex items-center justify-center py-4 absolute inset-0">
                <Loader2 class="size-6 animate-spin text-primary" />
            </div>
        {/if}
    </div>
</div>
