<script lang="ts">
    import { onMount } from "svelte";
    import { get } from "$lib/api";
    import type {
        PanelSession,
        Asset,
        PaginatedAssets,
    } from "$lib/components/panel/types";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import TimelineEntry from "$lib/components/panel/TimelineEntry.svelte";
    import { Activity, Clock } from "lucide-svelte";
    import { tick } from "svelte";

    let sessions = $state<PanelSession[]>([]);
    let selectedRunId = $state<string | null>(null);
    let assets = $state<Asset[]>([]);
    let loadingSessions = $state(true);
    let loadingAssets = $state(false);

    // Live mode polling
    let liveInterval: ReturnType<typeof setInterval> | null = null;
    let lastSeq = $state(0);
    let isLive = $state(false);

    // Auto-scroll
    let timelineContainer = $state<HTMLElement | null>(null);
    let autoScroll = $state(true);
    let hasOlderData = $state(true);
    let loadingOlder = $state(false);

    async function loadSessions() {
        loadingSessions = true;
        try {
            sessions = await get<PanelSession[]>("/panel/sessions");

            // One-time auto-select from URL param
            const urlRun = new URLSearchParams(window.location.search).get(
                "run",
            );
            if (urlRun && sessions.some((s) => s.run_id === urlRun)) {
                selectSession(urlRun);
            }
        } catch (e) {
            console.error("Failed to load sessions", e);
        } finally {
            loadingSessions = false;
        }
    }

    function isSessionLive(lastModified: string) {
        const modTime = new Date(lastModified).getTime();
        const now = Date.now();
        return now - modTime < 5 * 60 * 1000;
    }

    async function selectSession(runId: string) {
        if (selectedRunId === runId) return;

        selectedRunId = runId;
        assets = [];
        lastSeq = 0;
        autoScroll = true;
        hasOlderData = true;
        loadingOlder = false;

        const session = sessions.find((s) => s.run_id === runId);
        isLive = session ? isSessionLive(session.last_modified) : false;

        loadingAssets = true;
        await fetchLatestAssets();
        loadingAssets = false;

        // Scroll to bottom after initial load
        await tick();
        if (timelineContainer) {
            timelineContainer.scrollTop = timelineContainer.scrollHeight;
        }

        setupPolling();
    }

    // Initial load: fetch the latest N assets (no since/before = latest, ordered by seq DESC then reversed)
    async function fetchLatestAssets() {
        if (!selectedRunId) return;
        try {
            const result = await get<PaginatedAssets>(
                `/panel/${selectedRunId}/assets?limit=100`,
            );
            if (result.assets.length > 0) {
                assets = result.assets;
                lastSeq = assets[assets.length - 1].seq;
                hasOlderData = assets[0].seq > 1;
            } else {
                hasOlderData = false;
            }
        } catch (e) {
            console.error("Failed to fetch latest assets", e);
        }
    }

    // Load older assets when scrolling up
    async function fetchOlderAssets() {
        if (
            !selectedRunId ||
            loadingOlder ||
            !hasOlderData ||
            assets.length === 0
        )
            return;
        loadingOlder = true;

        const oldestSeq = assets[0].seq;
        const prevScrollHeight = timelineContainer?.scrollHeight ?? 0;

        try {
            const result = await get<PaginatedAssets>(
                `/panel/${selectedRunId}/assets?before=${oldestSeq}&limit=100`,
            );
            if (result.assets.length > 0) {
                assets = [...result.assets, ...assets];
                hasOlderData = result.assets[0].seq > 1;

                // Preserve scroll position after prepending
                await tick();
                if (timelineContainer) {
                    const newScrollHeight = timelineContainer.scrollHeight;
                    timelineContainer.scrollTop =
                        newScrollHeight - prevScrollHeight;
                }
            } else {
                hasOlderData = false;
            }
        } catch (e) {
            console.error("Failed to fetch older assets", e);
        } finally {
            loadingOlder = false;
        }
    }

    // Poll for new assets (forward direction)
    async function fetchNewAssets() {
        if (!selectedRunId) return;
        try {
            const params = new URLSearchParams();
            if (lastSeq) params.append("since", lastSeq.toString());
            const result = await get<PaginatedAssets>(
                `/panel/${selectedRunId}/assets?${params.toString()}`,
            );
            if (result.assets.length > 0) {
                assets = [...assets, ...result.assets];
                lastSeq = result.assets[result.assets.length - 1].seq;

                // Auto-scroll to bottom for new entries
                if (autoScroll && timelineContainer) {
                    tick().then(() => {
                        if (timelineContainer) {
                            timelineContainer.scrollTo({
                                top: timelineContainer.scrollHeight,
                                behavior: "smooth",
                            });
                        }
                    });
                }
            }
        } catch (e) {
            console.error("Failed to fetch assets", e);
        }
    }

    function setupPolling() {
        if (liveInterval) {
            clearInterval(liveInterval);
            liveInterval = null;
        }
        if (isLive) {
            liveInterval = setInterval(fetchNewAssets, 500);
        }
    }

    function handleTimelineScroll(e: Event) {
        const el = e.target as HTMLElement;
        const isAtBottom =
            el.scrollHeight - el.scrollTop <= el.clientHeight + 30;
        autoScroll = isAtBottom;

        // Trigger loading older data when scrolling near the top
        if (el.scrollTop < 200 && hasOlderData && !loadingOlder) {
            fetchOlderAssets();
        }
    }

    function formatTime(ts: string) {
        if (!ts) return "—";
        return new Date(ts).toLocaleString();
    }

    function formatSize(bytes: number) {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
        return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    }

    onMount(() => {
        loadSessions();
        return () => {
            if (liveInterval) clearInterval(liveInterval);
        };
    });
</script>

<div class="flex h-full w-full overflow-hidden bg-background">
    <!-- Left Sidebar: Sessions List -->
    <div class="w-64 border-r shrink-0 flex flex-col bg-muted/20">
        <div class="p-4 border-b shrink-0 flex items-center justify-between">
            <h2 class="font-semibold text-lg">Panel Sessions</h2>
            <Badge variant="secondary">{sessions.length}</Badge>
        </div>
        <!-- native scroll instead of ScrollArea to avoid click interception -->
        <div class="flex-1 overflow-y-auto">
            <div class="p-2 flex flex-col gap-2">
                {#if loadingSessions}
                    <div class="p-4 text-center text-sm text-muted-foreground">
                        Loading...
                    </div>
                {:else if sessions.length === 0}
                    <div class="p-4 text-center text-sm text-muted-foreground">
                        No panel data recorded yet.
                    </div>
                {:else}
                    {#each sessions as session}
                        {@const _isLive = isSessionLive(session.last_modified)}
                        <button
                            class="flex flex-col text-left p-3 rounded-md transition-colors border cursor-pointer {selectedRunId ===
                            session.run_id
                                ? 'bg-accent/50 border-ring/50'
                                : 'bg-card hover:bg-accent/30 border-transparent'} relative overflow-hidden"
                            onclick={() => selectSession(session.run_id)}
                        >
                            <div
                                class="flex items-center justify-between w-full mb-1"
                            >
                                <span
                                    class="font-mono text-xs font-semibold truncate pr-2"
                                >
                                    {session.run_id.split("-")[0]}
                                </span>
                                {#if _isLive}
                                    <span
                                        class="flex size-2 rounded-full bg-green-500 animate-pulse"
                                    ></span>
                                {:else}
                                    <Clock
                                        class="size-3 text-muted-foreground"
                                    />
                                {/if}
                            </div>
                            <div
                                class="text-xs text-muted-foreground mt-1 flex justify-between"
                            >
                                <span>{session.asset_count} assets</span>
                                <span
                                    >{formatSize(session.disk_size_bytes)}</span
                                >
                            </div>
                            <div
                                class="text-[10px] text-muted-foreground/70 mt-1 truncate"
                            >
                                {formatTime(session.last_modified)}
                            </div>
                        </button>
                    {/each}
                {/if}
            </div>
        </div>
    </div>

    <!-- Right Main Area: Timeline View -->
    <div class="flex-1 flex flex-col min-w-0 bg-background relative">
        {#if !selectedRunId}
            <div
                class="flex-1 flex flex-col items-center justify-center text-muted-foreground"
            >
                <Activity class="size-12 mb-4 opacity-20" />
                <p>Select a session from the sidebar to view panel data</p>
                <p class="text-xs mt-2 opacity-60">
                    Start a dataflow with dm-panel to see live data
                </p>
            </div>
        {:else}
            <!-- Header -->
            <div
                class="p-4 border-b shrink-0 flex items-center justify-between bg-card z-10 shadow-sm"
            >
                <div class="flex items-center gap-3">
                    <h2 class="font-semibold font-mono text-sm">
                        {selectedRunId}
                    </h2>
                    {#if isLive}
                        <Badge
                            variant="default"
                            class="bg-green-600/20 text-green-600 hover:bg-green-600/30"
                            >Live</Badge
                        >
                    {:else}
                        <Badge variant="secondary">Replay</Badge>
                    {/if}
                </div>
                <div class="text-xs text-muted-foreground">
                    {assets.length} assets loaded
                </div>
            </div>

            <!-- Timeline -->
            <div
                bind:this={timelineContainer}
                onscroll={handleTimelineScroll}
                class="flex-1 overflow-y-auto"
            >
                {#if loadingAssets && assets.length === 0}
                    <div class="w-full text-center py-12 text-muted-foreground">
                        Loading assets...
                    </div>
                {:else if assets.length === 0}
                    <div
                        class="m-6 text-center py-12 text-muted-foreground border border-dashed rounded-lg bg-muted/10"
                    >
                        No assets recorded in this session yet.
                    </div>
                {:else}
                    {#if loadingOlder}
                        <div
                            class="flex items-center justify-center gap-2 py-3 text-xs text-muted-foreground"
                        >
                            <span
                                class="size-3 animate-spin rounded-full border-2 border-current border-t-transparent"
                            ></span>
                            Loading older messages...
                        </div>
                    {:else if !hasOlderData}
                        <div
                            class="text-center py-3 text-[11px] text-muted-foreground/50"
                        >
                            — Beginning of session —
                        </div>
                    {/if}

                    <div class="divide-y divide-border/30">
                        {#each assets as asset (asset.seq)}
                            <TimelineEntry {asset} runId={selectedRunId} />
                        {/each}
                    </div>

                    {#if isLive}
                        <div
                            class="flex items-center justify-center gap-2 py-3 text-xs text-green-500/70"
                        >
                            <span
                                class="size-1.5 rounded-full bg-green-500 animate-pulse"
                            ></span>
                            Listening for new data...
                        </div>
                    {/if}
                {/if}
            </div>
        {/if}
    </div>
</div>
