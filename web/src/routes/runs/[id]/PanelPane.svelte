<script lang="ts">
    import { onMount, onDestroy, tick } from "svelte";
    import { get, post } from "$lib/api";
    import { ArrowDown } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import { PaneGroup, Pane, PaneResizer } from "paneforge";
    import PanelMessage from "./PanelMessage.svelte";
    import PanelWidgets from "./PanelWidgets.svelte";

    interface Props {
        runId: string;
        isRunActive: boolean;
    }

    let { runId, isRunActive }: Props = $props();

    let panelAssets = $state<any[]>([]);
    let panelInput = $state("");
    let panelLoading = $state(false);
    let sendingCommand = $state(false);
    let panelMessagesDiv = $state<HTMLElement | null>(null);
    let hasMoreHistory = $state(true);
    let loadingHistory = $state(false);
    let showScrollBottom = $state(false);
    let panelWidgets = $state<Record<string, any> | null>(null);

    let panelWs: WebSocket | null = null;
    let wsRetryTimeout: ReturnType<typeof setTimeout> | null = null;
    let usingWsFallback = $state(false);
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    // ── Data fetching ──

    async function fetchPanelAssets() {
        panelLoading = true;
        try {
            const response: any = await get(
                `/runs/${runId}/panel/assets?limit=50`,
            );
            const assets = Array.isArray(response)
                ? response
                : response?.assets || [];
            hasMoreHistory = assets.length === 50;
            panelAssets = assets;

            tick().then(() => {
                if (panelMessagesDiv) {
                    panelMessagesDiv.scrollTop = panelMessagesDiv.scrollHeight;
                }
            });
        } catch (e) {
            console.error("Panel assets fetch failed", e);
        } finally {
            panelLoading = false;
        }
    }

    async function fetchPanelWidgets() {
        try {
            const result = await get<Record<string, any>>(
                `/runs/${runId}/panel/widgets`,
            );
            if (result && Object.keys(result).length > 0) {
                panelWidgets = result;
            }
        } catch (e) {
            console.error("Panel widgets fetch failed", e);
        }
    }

    async function pollNewPanelAssets() {
        if (loadingHistory) return;

        let url = `/runs/${runId}/panel/assets?limit=50`;
        const newestSeq =
            panelAssets.length > 0
                ? panelAssets[panelAssets.length - 1].seq
                : null;
        if (newestSeq) {
            url += `&since=${newestSeq}`;
        }

        try {
            const response: any = await get(url);
            const newAssets = Array.isArray(response)
                ? response
                : response?.assets || [];
            if (newAssets.length > 0) {
                if (newestSeq) {
                    appendPanelAssets(newAssets);
                } else {
                    panelAssets = newAssets;
                    hasMoreHistory = newAssets.length === 50;
                    tick().then(() => {
                        if (panelMessagesDiv)
                            panelMessagesDiv.scrollTop =
                                panelMessagesDiv.scrollHeight;
                    });
                }
            }
        } catch (e) {
            console.error("Panel assets poll failed", e);
        }
    }

    async function loadOlderAssets() {
        if (loadingHistory || !hasMoreHistory || panelAssets.length === 0)
            return;
        loadingHistory = true;

        const oldestSeq = panelAssets[0].seq;
        const url = `/runs/${runId}/panel/assets?limit=50&before=${oldestSeq}`;

        try {
            const response: any = await get(url);
            const olderAssets = Array.isArray(response)
                ? response
                : response?.assets || [];

            if (olderAssets.length > 0) {
                const oldScrollHeight = panelMessagesDiv?.scrollHeight || 0;
                const oldScrollTop = panelMessagesDiv?.scrollTop || 0;

                panelAssets = [...olderAssets, ...panelAssets];
                hasMoreHistory = olderAssets.length === 50;

                tick().then(() => {
                    if (panelMessagesDiv) {
                        const newScrollHeight = panelMessagesDiv.scrollHeight;
                        panelMessagesDiv.scrollTop =
                            oldScrollTop + (newScrollHeight - oldScrollHeight);
                    }
                });
            } else {
                hasMoreHistory = false;
            }
        } catch (e) {
            console.error("Panel history fetch failed", e);
        } finally {
            loadingHistory = false;
        }
    }

    // ── WebSocket ──

    function getWsUrl(since: number): string {
        const proto = location.protocol === "https:" ? "wss:" : "ws:";
        return `${proto}//${location.host}/api/runs/${runId}/panel/ws?since=${since}`;
    }

    function connectPanelWs() {
        if (!isRunActive) return;

        const newestSeq =
            panelAssets.length > 0
                ? panelAssets[panelAssets.length - 1].seq
                : 0;

        const ws = new WebSocket(getWsUrl(newestSeq));
        panelWs = ws;
        usingWsFallback = false;

        ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                if (msg.type === "assets" && Array.isArray(msg.data) && msg.data.length > 0) {
                    appendPanelAssets(msg.data);
                }
            } catch (e) {
                console.error("Panel WS parse error", e);
            }
        };

        ws.onerror = () => {
            console.warn("Panel WS error, falling back to HTTP polling");
            ws.close();
        };

        ws.onclose = () => {
            panelWs = null;
            if (isRunActive) {
                usingWsFallback = true;
                wsRetryTimeout = setTimeout(() => {
                    if (isRunActive) connectPanelWs();
                }, 10000);
            }
        };
    }

    function appendPanelAssets(newAssets: any[]) {
        if (newAssets.length === 0) return;

        const isAtBottom =
            panelMessagesDiv &&
            Math.abs(
                panelMessagesDiv.scrollHeight -
                    panelMessagesDiv.clientHeight -
                    panelMessagesDiv.scrollTop,
            ) < 50;

        const newestSeq =
            panelAssets.length > 0
                ? panelAssets[panelAssets.length - 1].seq
                : 0;

        const filtered = newAssets.filter((a: any) => a.seq > newestSeq);
        if (filtered.length === 0) return;

        panelAssets = [...panelAssets, ...filtered];

        if (isAtBottom) {
            tick().then(() => {
                if (panelMessagesDiv)
                    panelMessagesDiv.scrollTop = panelMessagesDiv.scrollHeight;
            });
        }
    }

    function closePanelWs() {
        if (panelWs) {
            panelWs.close();
            panelWs = null;
        }
        if (wsRetryTimeout) {
            clearTimeout(wsRetryTimeout);
            wsRetryTimeout = null;
        }
    }

    // ── UI interactions ──

    function handlePanelScroll(e: Event) {
        const target = e.target as HTMLElement;
        if (target.scrollTop < 100) {
            loadOlderAssets();
        }
        const isAtBottom =
            Math.abs(
                target.scrollHeight - target.clientHeight - target.scrollTop,
            ) < 50;
        showScrollBottom = !isAtBottom;
    }

    function scrollToBottom() {
        if (panelMessagesDiv) {
            panelMessagesDiv.scrollTo({
                top: panelMessagesDiv.scrollHeight,
                behavior: "smooth",
            });
            showScrollBottom = false;
        }
    }

    async function sendCommand() {
        if (!panelInput.trim()) return;
        sendingCommand = true;
        try {
            await post(`/runs/${runId}/panel/commands`, {
                command: panelInput,
            });
            panelInput = "";
        } catch (e: any) {
            console.error("Command send failed", e);
            alert("Failed to send command: " + e.message);
        } finally {
            sendingCommand = false;
        }
    }

    // ── Lifecycle ──

    onMount(() => {
        fetchPanelAssets().then(() => {
            connectPanelWs();
        });
        fetchPanelWidgets();

        pollInterval = setInterval(() => {
            if (isRunActive && usingWsFallback) {
                pollNewPanelAssets();
            }
        }, 3000);
    });

    onDestroy(() => {
        if (pollInterval) clearInterval(pollInterval);
        closePanelWs();
    });
</script>

<div
    class="bg-muted/30 px-4 border-b flex items-center shrink-0 h-11"
>
    <span
        class="text-[11px] font-semibold tracking-wider uppercase text-muted-foreground"
        >Panel Interface</span
    >
</div>

<PaneGroup direction="vertical" autoSaveId="dora-panel-vertical-split" class="flex-1 min-h-0 relative flex flex-col">
    <!-- Top: Messages Feed -->
    <Pane id="messages" defaultSize={70} minSize={20} class="flex flex-col relative overflow-hidden bg-background">
        <div
            class="flex-1 overflow-y-auto p-4 flex flex-col gap-4 relative"
            bind:this={panelMessagesDiv}
            onscroll={handlePanelScroll}
        >
            {#if panelLoading && panelAssets.length === 0}
                <div class="text-sm text-muted-foreground text-center my-auto">
                    Loading panel connection...
                </div>
            {:else if panelAssets.length === 0}
                <div class="text-sm text-muted-foreground text-center my-auto">
                    No panel assets received yet.
                </div>
            {:else}
                {#if hasMoreHistory}
                    <div class="py-2 text-center text-xs text-muted-foreground flex items-center justify-center shrink-0 min-h-[32px]">
                        {#if loadingHistory}
                            <div class="animate-pulse">Loading older messages...</div>
                        {/if}
                    </div>
                {:else}
                    <div class="py-4 text-center text-xs text-muted-foreground opacity-50 shrink-0">
                        -- Start of conversation --
                    </div>
                {/if}

                {#each panelAssets as asset}
                    <PanelMessage {asset} {runId} />
                {/each}
            {/if}
        </div>

        <!-- Floating Scroll to Bottom Button -->
        {#if showScrollBottom}
            <div class="absolute bottom-4 right-6 z-20 pointer-events-none">
                <Button
                    variant="secondary"
                    size="icon"
                    class="h-8 w-8 rounded-full shadow-md border pointer-events-auto bg-background/90 backdrop-blur"
                    onclick={scrollToBottom}
                    title="Scroll to latest message"
                >
                    <ArrowDown class="h-4 w-4" />
                </Button>
            </div>
        {/if}
    </Pane>

    <PaneResizer class="h-px relative bg-border hover:bg-primary/50 active:bg-primary/80 transition-all cursor-row-resize z-20 after:absolute after:inset-x-0 after:-inset-y-2" />

    <!-- Bottom: Command / Widgets Area -->
    <Pane id="controls" defaultSize={30} minSize={15} maxSize={50} class="flex flex-col overflow-hidden bg-background/95 backdrop-blur shadow-[0_-8px_15px_-3px_rgba(0,0,0,0.05)] border-t z-10 relative">
        <div class="flex-1 overflow-y-auto p-4 flex flex-col">
            {#if panelWidgets}
                <PanelWidgets
                    {runId}
                    widgets={panelWidgets}
                    disabled={!isRunActive}
                />
            {:else}
                <form
                    onsubmit={(e) => {
                        e.preventDefault();
                        sendCommand();
                    }}
                    class="flex items-center gap-2 relative w-full my-auto"
                >
                    <div class="flex-1 relative group">
                        <input
                            type="text"
                            bind:value={panelInput}
                            placeholder="Type a command..."
                            class="w-full h-10 pl-3 pr-10 rounded-lg border bg-background text-sm focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 transition-shadow"
                            disabled={sendingCommand || !isRunActive}
                        />
                        <Button
                            type="submit"
                            size="icon"
                            variant="ghost"
                            class="absolute right-1 top-1 bottom-1 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
                            disabled={sendingCommand || !panelInput.trim() || !isRunActive}
                            title="Send command (Enter)"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-send"><path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/></svg>
                        </Button>
                    </div>
                </form>
            {/if}
        </div>
    </Pane>
</PaneGroup>
