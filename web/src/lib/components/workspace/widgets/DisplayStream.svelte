<script lang="ts">
    import { untrack } from "svelte";
    import { get } from "$lib/api";
    import DisplayMessageItem from "./DisplayMessageItem.svelte";
    import { Filter, Loader2 } from "lucide-svelte";

    let {
        node,
        api,
        runId,
        streams = [],
        nodes = [],
        onConfigChange,
    } = $props<{
        node: any;
        api: any;
        runId: string;
        streams: any[];
        nodes?: any[];
        onConfigChange?: () => void;
    }>();

    let subscribedSourceId = $state(node.config.subscribedSourceId || "");

    let allMessages = $state<any[]>([]);
    let fetching = $state(false);
    let fetchingOld = $state(false);
    let hasMoreOld = $state(true);

    let container = $state<HTMLElement | null>(null);
    let isUserScrolling = $state(false);

    let oldestSeq = $derived(
        allMessages.length > 0 ? allMessages[0].seq : null,
    );
    let newestSeq = $derived(
        allMessages.length > 0 ? allMessages[allMessages.length - 1].seq : null,
    );

    async function loadInitial() {
        if (!runId || fetching) return;
        fetching = true;
        try {
            let url = `/runs/${runId}/interaction/stream/messages?limit=50&desc=true`;
            if (subscribedSourceId) url += `&source_id=${subscribedSourceId}`;

            const res: any = await get(url);
            if (res && res.messages) {
                allMessages = res.messages;
                hasMoreOld = res.messages.length === 50;
                scrollToBottom(true);
            }
        } catch (e) {
            console.error("Failed to load initial messages", e);
        } finally {
            fetching = false;
        }
    }

    async function loadNewMessages() {
        if (!runId || fetching || !newestSeq) return;
        fetching = true;
        try {
            let url = `/runs/${runId}/interaction/stream/messages?after_seq=${newestSeq}&limit=50`;
            if (subscribedSourceId) url += `&source_id=${subscribedSourceId}`;

            const res: any = await get(url);
            if (res && res.messages && res.messages.length > 0) {
                allMessages = [...allMessages, ...res.messages];

                if (!isUserScrolling) {
                    scrollToBottom();
                }
            }
        } catch (e) {
            console.error("Failed to pull new messages", e);
        } finally {
            fetching = false;
        }
    }

    async function loadOldMessages() {
        if (!runId || fetchingOld || !oldestSeq || !hasMoreOld) return;
        fetchingOld = true;

        let previousHeight = container ? container.scrollHeight : 0;

        try {
            let url = `/runs/${runId}/interaction/stream/messages?before_seq=${oldestSeq}&limit=50&desc=true`;
            if (subscribedSourceId) url += `&source_id=${subscribedSourceId}`;

            const res: any = await get(url);
            if (res && res.messages && res.messages.length > 0) {
                hasMoreOld = res.messages.length === 50;
                allMessages = [...res.messages, ...allMessages];

                if (container) {
                    setTimeout(() => {
                        if (container) {
                            container.scrollTop =
                                container.scrollHeight - previousHeight;
                        }
                    }, 0);
                }
            } else {
                hasMoreOld = false;
            }
        } catch (e) {
            console.error("Failed to load old messages", e);
        } finally {
            fetchingOld = false;
        }
    }

    function handleSourceChange(e: Event) {
        const id = (e.currentTarget as HTMLSelectElement).value;
        subscribedSourceId = id;
        if (!node.config) node.config = {};
        node.config.subscribedSourceId = id;
        if (onConfigChange) onConfigChange();

        allMessages = [];
        hasMoreOld = true;
        loadInitial();
    }

    function scrollToBottom(force = false) {
        if (container) {
            setTimeout(
                () => {
                    if (container) container.scrollTop = container.scrollHeight;
                },
                force ? 50 : 10,
            );
        }
    }

    function handleScroll() {
        if (!container) return;
        const { scrollTop, scrollHeight, clientHeight } = container;
        isUserScrolling =
            Math.abs(scrollHeight - clientHeight - scrollTop) > 40;

        if (scrollTop < 10 && !fetchingOld && hasMoreOld) {
            loadOldMessages();
        }
    }

    // Ping mechanism triggered by backend displays variable update
    $effect(() => {
        if (streams) {
            untrack(() => {
                if (allMessages.length > 0) {
                    loadNewMessages();
                } else {
                    loadInitial();
                }
            });
        }
    });
</script>

<div class="flex flex-col h-full w-full overflow-hidden bg-background">
    <!-- Header -->
    <div
        class="px-3 h-8 border-b bg-muted/20 flex items-center justify-between shrink-0"
    >
        <div class="flex items-center gap-2">
            <span
                class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider"
                >Message</span
            >
        </div>
        <div class="flex items-center gap-2">
            <Filter class="size-3 text-muted-foreground" />
            <select
                class="text-[11px] font-mono text-muted-foreground bg-muted hover:bg-muted/80 px-1 py-0.5 rounded border-0 outline-none ring-0 focus:ring-1 focus:ring-primary cursor-pointer max-w-[140px] truncate"
                value={subscribedSourceId}
                onchange={handleSourceChange}
            >
                <option value="">(All Sources)</option>
                {#each streams as dItem (dItem.node_id)}
                    <option value={dItem.node_id}>{dItem.node_id}</option>
                {/each}
            </select>
        </div>
    </div>

    <!-- Messages Container -->
    <div
        class="flex-1 overflow-y-auto p-4 space-y-4 bg-muted/10 relative"
        bind:this={container}
        onscroll={handleScroll}
    >
        {#if fetchingOld}
            <div
                class="flex items-center justify-center p-2 text-muted-foreground/60 w-full mb-4"
            >
                <Loader2 class="size-4 animate-spin" />
            </div>
        {:else if !hasMoreOld && allMessages.length > 0}
            <div
                class="flex items-center justify-center py-2 text-[10px] text-muted-foreground/40 uppercase tracking-widest w-full mb-4"
            >
                -- Top of History --
            </div>
        {/if}

        {#if allMessages.length === 0 && !fetching}
            <div
                class="absolute inset-0 flex flex-col items-center justify-center text-sm text-muted-foreground/60 tracking-wider"
            >
                <span class="font-mono">> No Messages</span>
            </div>
        {/if}

        <div class="flex flex-col gap-4 justify-end min-h-full">
            {#each allMessages as entry (entry.node_id + "_" + entry.seq)}
                <DisplayMessageItem {runId} {entry} />
            {/each}
        </div>

        {#if fetching && allMessages.length === 0}
            <div class="flex items-center justify-center py-4 absolute inset-0">
                <Loader2 class="size-6 animate-spin text-primary" />
            </div>
        {/if}
    </div>
</div>
