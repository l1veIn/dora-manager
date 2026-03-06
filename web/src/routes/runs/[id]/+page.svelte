<script lang="ts">
    import { page } from "$app/stores";
    import { onMount, onDestroy, tick } from "svelte";
    import { get, post } from "$lib/api";
    import { goto } from "$app/navigation";
    import { ChevronLeft, ArrowDown } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import { PaneGroup, Pane, PaneResizer } from "paneforge";

    import RunHeader from "$lib/components/runs/RunHeader.svelte";
    import RunFailureBanner from "$lib/components/runs/RunFailureBanner.svelte";
    import RunSummaryCard from "$lib/components/runs/RunSummaryCard.svelte";
    import RunNodeList from "$lib/components/runs/RunNodeList.svelte";
    import RunLogViewer from "$lib/components/runs/RunLogViewer.svelte";
    import PanelMessage from "$lib/components/panel/PanelMessage.svelte";

    let runId = $derived($page.params.id);

    let run = $state<any>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    let selectedNodeId = $state<string>("");

    let panelAssets = $state<any[]>([]);
    let panelInput = $state("");
    let panelLoading = $state(false);
    let sendingCommand = $state(false);
    let panelMessagesDiv = $state<HTMLElement | null>(null);
    let hasMoreHistory = $state(true);
    let loadingHistory = $state(false);
    let showScrollBottom = $state(false);
    let stoppingRun = $state(false);

    let isRunActive = $derived(run?.status === "running");

    async function fetchRunDetail() {
        if (!runId) return;
        try {
            const result = await get(`/runs/${runId}`);
            run = result;
            if (run?.nodes?.length > 0 && !selectedNodeId) {
                const nonEmpty = run.nodes.find((n: any) => n.log_size > 0);
                selectedNodeId = nonEmpty ? nonEmpty.id : run.nodes[0].id;
            }
        } catch (e: any) {
            console.error("Failed to fetch run", e);
            error = e.message || "Run not found";
        } finally {
            loading = false;
        }
    }

    async function stopRun() {
        if (!runId) return;
        stoppingRun = true;
        try {
            await post(`/runs/${runId}/stop`);
            let maxAttempts = 10;
            while (maxAttempts > 0) {
                await fetchRunDetail();
                if (run?.status !== "running") break;
                await new Promise((r) => setTimeout(r, 1000));
                maxAttempts--;
            }
        } catch (e: any) {
            alert(`Failed to stop run: ${e.message}`);
        } finally {
            stoppingRun = false;
        }
    }

    async function fetchPanelAssets() {
        if (!runId || !run?.has_panel) return;
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

    async function pollNewPanelAssets() {
        if (!runId || !run?.has_panel || loadingHistory) return;

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
                const isAtBottom =
                    panelMessagesDiv &&
                    Math.abs(
                        panelMessagesDiv.scrollHeight -
                            panelMessagesDiv.clientHeight -
                            panelMessagesDiv.scrollTop,
                    ) < 50;

                if (newestSeq) {
                    panelAssets = [...panelAssets, ...newAssets];
                } else {
                    panelAssets = newAssets;
                    hasMoreHistory = newAssets.length === 50;
                }

                if (isAtBottom || !newestSeq) {
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
        if (
            !runId ||
            !run?.has_panel ||
            loadingHistory ||
            !hasMoreHistory ||
            panelAssets.length === 0
        )
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
        if (!panelInput.trim() || !runId) return;
        sendingCommand = true;
        try {
            await post(`/runs/${runId}/panel/commands`, {
                command: panelInput,
            });
            panelInput = "";
            fetchPanelAssets(); // immediately refresh
        } catch (e: any) {
            console.error("Command send failed", e);
            alert("Failed to send command: " + e.message);
        } finally {
            sendingCommand = false;
        }
    }

    let mainPolling: ReturnType<typeof setInterval> | null = null;

    onMount(() => {
        fetchRunDetail().then(() => {
            if (run?.has_panel) fetchPanelAssets();
        });

        mainPolling = setInterval(() => {
            if (isRunActive) {
                fetchRunDetail();
                if (run?.has_panel) pollNewPanelAssets();
            } else if (mainPolling) {
                clearInterval(mainPolling);
            }
        }, 3000);
    });

    onDestroy(() => {
        if (mainPolling) clearInterval(mainPolling);
    });
</script>

<div class="h-full w-full flex flex-col overflow-hidden bg-background">
    <!-- Slim Global Header -->
    <div class="shrink-0">
        <RunHeader {run} onStop={stopRun} isStopping={stoppingRun} />
    </div>

    {#if loading}
        <div
            class="flex-1 flex items-center justify-center text-muted-foreground"
        >
            <div class="animate-pulse">Loading workspace...</div>
        </div>
    {:else if error || !run}
        <div
            class="flex-1 flex flex-col items-center justify-center gap-4 text-center"
        >
            <h2 class="text-2xl font-semibold">Run Not Found</h2>
            <p class="text-muted-foreground">{error}</p>
            <Button onclick={() => goto("/runs")}>Return to Runs list</Button>
        </div>
    {:else}
        {#if run.failure_node}
            <RunFailureBanner
                failureNode={run.failure_node}
                failureMessage={run.failure_message}
            />
        {/if}

        <div class="flex-1 min-h-0 flex w-full">
            <!-- Left Pane: Navigation & Status Sidebar -->
            <aside
                class="w-[300px] shrink-0 border-r bg-muted/10 flex flex-col overflow-y-auto"
            >
                <RunSummaryCard {run} />
                <RunNodeList nodes={run.nodes || []} bind:selectedNodeId />
            </aside>

            <!-- Middle and Right Resizable Panes -->
            <PaneGroup
                direction="horizontal"
                autoSaveId="dora-runs-ide-layout"
                class="flex-1 min-w-0 flex h-full overflow-hidden relative"
            >
                <Pane
                    defaultSize={run.has_panel ? 33 : 100}
                    minSize={20}
                    class="bg-background flex flex-col relative text-foreground h-full overflow-hidden"
                >
                    <RunLogViewer
                        runId={runId || ""}
                        nodeId={selectedNodeId}
                        {isRunActive}
                    />
                </Pane>

                {#if run.has_panel}
                    <PaneResizer
                        class="w-px relative bg-border hover:bg-primary/50 active:bg-primary/80 transition-all cursor-col-resize z-20 after:absolute after:inset-y-0 after:-inset-x-2"
                    />

                    <!-- Right Pane: Interaction Panel -->
                    <Pane
                        defaultSize={67}
                        minSize={25}
                        class="bg-card flex flex-col h-full z-10 shadow-xl overflow-hidden"
                    >
                        <div
                            class="bg-muted/30 px-4 border-b flex items-center shrink-0 h-11"
                        >
                            <span
                                class="text-[11px] font-semibold tracking-wider uppercase text-muted-foreground"
                                >Panel Interface</span
                            >
                        </div>

                        <!-- Panel Messages Feed -->
                        <div
                            class="flex-1 overflow-y-auto p-4 flex flex-col gap-4 relative"
                            bind:this={panelMessagesDiv}
                            onscroll={handlePanelScroll}
                        >
                            {#if panelLoading && panelAssets.length === 0}
                                <div
                                    class="text-sm text-muted-foreground text-center my-auto"
                                >
                                    Loading panel connection...
                                </div>
                            {:else if panelAssets.length === 0}
                                <div
                                    class="text-sm text-muted-foreground text-center my-auto"
                                >
                                    No panel assets received yet.
                                </div>
                            {:else}
                                {#if hasMoreHistory}
                                    <div
                                        class="py-2 text-center text-xs text-muted-foreground flex items-center justify-center shrink-0 min-h-[32px]"
                                    >
                                        {#if loadingHistory}
                                            <div class="animate-pulse">
                                                Loading older messages...
                                            </div>
                                        {/if}
                                    </div>
                                {:else}
                                    <div
                                        class="py-4 text-center text-xs text-muted-foreground opacity-50 shrink-0"
                                    >
                                        -- Start of conversation --
                                    </div>
                                {/if}

                                {#each panelAssets as asset}
                                    <PanelMessage {asset} runId={runId || ""} />
                                {/each}
                            {/if}
                        </div>

                        <!-- Floating Scroll to Bottom Button -->
                        {#if showScrollBottom}
                            <div
                                class="absolute bottom-[84px] right-6 z-20 pointer-events-none"
                            >
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

                        <!-- Floating Panel Input -->
                        <div
                            class="p-4 border-t bg-card/80 backdrop-blur-md shrink-0"
                        >
                            <form
                                onsubmit={(e) => {
                                    e.preventDefault();
                                    sendCommand();
                                }}
                                class="flex items-center gap-2 relative shadow-sm rounded-md"
                            >
                                <input
                                    type="text"
                                    bind:value={panelInput}
                                    placeholder="Command panel..."
                                    class="flex-1 h-10 pl-3 pr-20 rounded-md border bg-background text-sm focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 transition-shadow"
                                    disabled={sendingCommand || !isRunActive}
                                />
                                <Button
                                    type="submit"
                                    size="sm"
                                    class="absolute right-1 top-1 bottom-1 h-8 rounded-sm shadow-none"
                                    disabled={sendingCommand ||
                                        !panelInput.trim() ||
                                        !isRunActive}
                                >
                                    Send
                                </Button>
                            </form>
                        </div>
                    </Pane>
                {/if}
            </PaneGroup>
        </div>
    {/if}
</div>
