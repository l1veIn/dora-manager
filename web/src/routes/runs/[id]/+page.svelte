<script lang="ts">
    import { page } from "$app/stores";
    import { onMount, onDestroy } from "svelte";
    import { browser } from "$app/environment";
    import { get, post } from "$lib/api";
    import { goto } from "$app/navigation";
    import { Button } from "$lib/components/ui/button/index.js";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { Plus, PanelLeftClose, PanelLeftOpen, X } from "lucide-svelte";

    import RunHeader from "./RunHeader.svelte";
    import RunFailureBanner from "./RunFailureBanner.svelte";
    import RunSummaryCard from "./RunSummaryCard.svelte";
    import RunNodeList from "./RunNodeList.svelte";

    import Workspace from "$lib/components/workspace/Workspace.svelte";
    import { getPanelDefinition } from "$lib/components/workspace/panels/registry";
    import {
        type WorkspaceGridItem,
        getDefaultLayout,
        mutateTreeInjectTerminal,
        generateId,
        normalizeWorkspaceLayout,
    } from "$lib/components/workspace/types";

    let runId = $derived($page.params.id);

    let run = $state<any>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let metrics = $state<any>(null);
    let snapshots = $state<any[]>([]);
    let inputValues = $state<Record<string, any>>({});

    let selectedNodeId = $state<string>("");
    let workspaceLayout = $state<WorkspaceGridItem[]>(getDefaultLayout());
    let workspaceLoaded = false;
    type StopRequestState = {
        phase: "idle" | "pending" | "delayed";
        requestedAt: number | null;
        error: string | null;
    };

    const STOP_DELAYED_THRESHOLD_MS = 15000;

    let stopRequest = $state<StopRequestState>({
        phase: "idle",
        requestedAt: null,
        error: null,
    });
    let messageSocket: WebSocket | null = null;
    let reconnectMessageSocket: ReturnType<typeof setTimeout> | null = null;
    let snapshotRefreshInFlight: Promise<void> | null = null;
    let inputValuesRefreshInFlight: Promise<void> | null = null;
    let latestInputSeq = $state(0);
    let messageRefreshToken = $state(0);
    let isRunSidebarOpen = $state(true);
    let interactionNoticeDismissed = $state(false);
    const panelOptions: Array<{ type: "chart" | "message" | "input" | "video" | "terminal"; label: string }> = [
        { type: "message", label: "Message" },
        { type: "input", label: "Input" },
        { type: "chart", label: "Chart" },
        { type: "video", label: "Plyr" },
        { type: "terminal", label: "Terminal" },
    ];

    function sidebarStorageKey() {
        return run?.name ? `dm-run-sidebar-open-${run.name}` : null;
    }

    function interactionNoticeStorageKey(name: string | undefined | null) {
        return name ? `dm-run-interaction-notice-dismissed-${name}` : null;
    }

    // Layout persistence
    function handleLayoutChange(newLayout: WorkspaceGridItem[]) {
        workspaceLayout = newLayout;
        if (run?.name && browser) {
            localStorage.setItem(
                `dm-workspace-layout-${run.name}`,
                JSON.stringify(newLayout),
            );
        }
    }

    function toggleRunSidebar() {
        isRunSidebarOpen = !isRunSidebarOpen;
        const key = sidebarStorageKey();
        if (key && browser) {
            localStorage.setItem(key, String(isRunSidebarOpen));
        }
    }

    function addWidget(type: "message" | "input" | "chart" | "video" | "terminal") {
        let maxY = 0;
        for (let item of workspaceLayout) {
            maxY = Math.max(maxY, item.y + item.h);
        }
        workspaceLayout = [
            ...workspaceLayout,
            {
                id: generateId(),
                widgetType: type,
                config: { ...getPanelDefinition(type).defaultConfig },
                x: 0,
                y: maxY,
                w: 6,
                h: 4,
            },
        ];
        handleLayoutChange(workspaceLayout);
    }

    function openNodeTerminal(id: string) {
        selectedNodeId = id;

        // Find existing terminal for this node
        let targetTx = workspaceLayout.find(
            (item) =>
                item.widgetType === "terminal" && item.config?.nodeId === id,
        );

        if (!targetTx) {
            // Find any existing terminal to recycle
            let anyTx = workspaceLayout.find(
                (item) => item.widgetType === "terminal",
            );
            if (anyTx) {
                if (!anyTx.config) anyTx.config = {};
                anyTx.config.nodeId = id;
                targetTx = anyTx;
                handleLayoutChange(workspaceLayout);
            } else {
                // Append a new terminal
                workspaceLayout = mutateTreeInjectTerminal(workspaceLayout, id);
                handleLayoutChange(workspaceLayout);
                targetTx = workspaceLayout.find(
                    (item) =>
                        item.widgetType === "terminal" &&
                        item.config?.nodeId === id,
                );
            }
        }

        if (targetTx) {
            const txId = targetTx.id;
            setTimeout(() => {
                const el = document.querySelector(
                    `[gs-id="${txId}"]`,
                ) as HTMLElement;
                if (el) {
                    // Focus & animate frame jump
                    el.scrollIntoView({ behavior: "smooth", block: "center" });
                    const wrapper = el.querySelector(
                        ".grid-stack-item-content > div",
                    ) as HTMLElement;
                    if (wrapper) {
                        wrapper.classList.remove(
                            "ring-offset-2",
                            "ring-2",
                            "ring-primary/80",
                        );
                        // forced reflow trick
                        void wrapper.offsetWidth;
                        wrapper.classList.add(
                            "transition-all",
                            "duration-500",
                            "ring-offset-2",
                            "ring-2",
                            "ring-primary/80",
                        );
                        setTimeout(
                            () =>
                                wrapper.classList.remove(
                                    "ring-offset-2",
                                    "ring-2",
                                    "ring-primary/80",
                                ),
                            1500,
                        );
                    }
                }
            }, 100); // slight delay to allow Svelte DOM flush for new components
        }
    }

    let isRunActive = $derived(run?.status === "running");
    let shouldPollRun = $derived(isRunActive || stopRequest.requestedAt !== null);
    let hasInteraction = $derived((snapshots?.length ?? 0) > 0);
    let hasWidgetInputs = $derived(
        (snapshots ?? []).some((snapshot: any) => snapshot.tag === "widgets"),
    );
    let showInteractionNotice = $derived(
        hasWidgetInputs && !interactionNoticeDismissed,
    );

    function clearStopRequest() {
        stopRequest = {
            phase: "idle",
            requestedAt: null,
            error: null,
        };
    }

    function syncStopRequest(nextRun: any) {
        const persistedRequestedAt = nextRun?.stop_requested_at
            ? Date.parse(nextRun.stop_requested_at)
            : null;
        const effectiveRequestedAt = persistedRequestedAt ?? stopRequest.requestedAt;

        if (!effectiveRequestedAt) {
            if (stopRequest.error) {
                stopRequest = {
                    phase: "idle",
                    requestedAt: null,
                    error: stopRequest.error,
                };
            }
            return;
        }

        if (nextRun?.status && nextRun.status !== "running") {
            clearStopRequest();
            return;
        }

        const nextPhase =
            Date.now() - effectiveRequestedAt >= STOP_DELAYED_THRESHOLD_MS
                ? "delayed"
                : "pending";

        if (
            stopRequest.phase !== nextPhase ||
            stopRequest.error ||
            stopRequest.requestedAt !== effectiveRequestedAt
        ) {
            stopRequest = {
                phase: nextPhase,
                requestedAt: effectiveRequestedAt,
                error: null,
            };
        }
    }

    function dismissInteractionNotice() {
        interactionNoticeDismissed = true;
        const key = interactionNoticeStorageKey(run?.name);
        if (key && browser) {
            localStorage.setItem(key, "true");
        }
    }

    // ── Data fetching ──

    async function fetchRunDetail() {
        if (!runId) return;
        try {
            const result = await get(
                `/runs/${runId}${shouldPollRun || loading ? "?include_metrics=true" : ""}`,
            );
            run = result;
            metrics = (result as any)?.metrics ?? null;
            syncStopRequest(result);
            if (run?.nodes?.length > 0 && !workspaceLoaded) {
                // Restore layout on first run load
                workspaceLoaded = true;
                const saved = localStorage.getItem(
                    `dm-workspace-layout-${run.name}`,
                );
                if (saved) {
                    try {
                        const parsed = JSON.parse(saved);
                        if (Array.isArray(parsed)) {
                            workspaceLayout = normalizeWorkspaceLayout(parsed);
                        } else {
                            console.warn(
                                "Discarding old Workspace layout version from LocalStorage",
                            );
                        }
                    } catch (e) {}
                }

                const sidebarKey = sidebarStorageKey();
                if (browser && sidebarKey) {
                    const savedSidebar = localStorage.getItem(sidebarKey);
                    if (savedSidebar !== null) {
                        isRunSidebarOpen = savedSidebar === "true";
                    }
                }

                const interactionKey = interactionNoticeStorageKey(run?.name);
                if (browser && interactionKey) {
                    interactionNoticeDismissed =
                        localStorage.getItem(interactionKey) === "true";
                }
            }
        } catch (e: any) {
            console.error("Failed to fetch run", e);
            error = e.message || "Run not found";
        } finally {
            loading = false;
        }
    }

    async function fetchSnapshots() {
        if (!runId) return;
        if (snapshotRefreshInFlight) {
            return snapshotRefreshInFlight;
        }

        snapshotRefreshInFlight = (async () => {
            try {
                snapshots = await get(`/runs/${runId}/messages/snapshots`);
            } catch (e) {
                console.error("Failed to fetch message snapshots", e);
            } finally {
                snapshotRefreshInFlight = null;
            }
        })();

        return snapshotRefreshInFlight;
    }

    async function fetchInputValues() {
        if (!runId) return;
        if (inputValuesRefreshInFlight) {
            return inputValuesRefreshInFlight;
        }

        inputValuesRefreshInFlight = (async () => {
            try {
                const response: any = await get(`/runs/${runId}/messages?tag=input&limit=5000`);
                const nextValues: Record<string, any> = {};
                latestInputSeq = 0;
                for (const message of response.messages ?? []) {
                    const key = `${message.payload?.to}:${message.payload?.output_id}`;
                    nextValues[key] = message.payload?.value;
                    latestInputSeq = Math.max(latestInputSeq, message.seq ?? 0);
                }
                inputValues = nextValues;
            } catch (e) {
                console.error("Failed to fetch input values", e);
            } finally {
                inputValuesRefreshInFlight = null;
            }
        })();

        return inputValuesRefreshInFlight;
    }

    async function fetchNewInputValues() {
        if (!runId) return;
        try {
            const response: any = await get(
                `/runs/${runId}/messages?tag=input&after_seq=${latestInputSeq}&limit=500`,
            );
            if ((response.messages ?? []).length === 0) {
                return;
            }
            inputValues = { ...inputValues };
            for (const message of response.messages ?? []) {
                const key = `${message.payload?.to}:${message.payload?.output_id}`;
                inputValues[key] = message.payload?.value;
                latestInputSeq = Math.max(latestInputSeq, message.seq ?? 0);
            }
        } catch (e) {
            console.error("Failed to fetch incremental input values", e);
        }
    }

    async function emitMessage(message: {
        from: string;
        tag: string;
        payload: any;
        timestamp?: number;
    }) {
        if (!runId) return;
        await post(`/runs/${runId}/messages`, message);
        if (message.tag === "input") {
            await fetchNewInputValues();
        }
        messageRefreshToken += 1;
    }

    async function stopRun() {
        if (!runId || stopRequest.requestedAt) return;
        stopRequest = {
            phase: "pending",
            requestedAt: Date.now(),
            error: null,
        };
        try {
            await post(`/runs/${runId}/stop`);
            await fetchRunDetail();

            let burstAttempts = 4;
            while (burstAttempts > 0 && stopRequest.requestedAt && run?.status === "running") {
                await new Promise((r) => setTimeout(r, 1000));
                await fetchRunDetail();
                burstAttempts--;
            }
        } catch (e: any) {
            stopRequest = {
                phase: "idle",
                requestedAt: null,
                error: e.message || "Failed to request stop.",
            };
        }
    }

    // ── Lifecycle ──

    let mainPolling: ReturnType<typeof setInterval> | null = null;

    function scheduleMessageSocketReconnect() {
        if (!browser || reconnectMessageSocket || !runId) return;
        reconnectMessageSocket = setTimeout(() => {
            reconnectMessageSocket = null;
            connectMessageSocket();
        }, 1000);
    }

    function closeMessageSocket() {
        if (reconnectMessageSocket) {
            clearTimeout(reconnectMessageSocket);
            reconnectMessageSocket = null;
        }
        if (messageSocket) {
            messageSocket.onopen = null;
            messageSocket.onmessage = null;
            messageSocket.onerror = null;
            messageSocket.onclose = null;
            messageSocket.close();
            messageSocket = null;
        }
    }

    function connectMessageSocket() {
        if (!browser || !runId) return;

        closeMessageSocket();

        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const socket = new WebSocket(
            `${protocol}//${window.location.host}/api/runs/${runId}/messages/ws`,
        );

        socket.onmessage = async (event) => {
            const notification = JSON.parse(event.data);
            await fetchSnapshots();
            if (notification.tag === "input") {
                await fetchNewInputValues();
            }
            messageRefreshToken += 1;
        };
        socket.onerror = () => {
            socket.close();
        };
        socket.onclose = () => {
            if (messageSocket === socket) {
                messageSocket = null;
            }
            scheduleMessageSocketReconnect();
        };

        messageSocket = socket;
    }

    onMount(() => {
        fetchRunDetail();
        fetchSnapshots();
        fetchInputValues();
        connectMessageSocket();
        mainPolling = setInterval(() => {
            if (shouldPollRun) {
                fetchRunDetail();
            } else {
                metrics = null;
            }
        }, 3000);
    });

    onDestroy(() => {
        if (mainPolling) clearInterval(mainPolling);
        closeMessageSocket();
    });
</script>

<div class="h-full w-full flex flex-col overflow-hidden bg-background">
    <!-- Slim Global Header -->
    <div class="shrink-0">
        <RunHeader {run} onStop={stopRun} stopRequest={stopRequest} />
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

        <div class="relative flex-1 min-h-0 flex w-full">
            <!-- Left Pane: Navigation & Status Sidebar -->
            {#if isRunSidebarOpen}
                <aside
                    class="w-[300px] shrink-0 border-r bg-muted/10 flex flex-col overflow-y-auto"
                >
                    <RunSummaryCard {run} {metrics} />
                    <RunNodeList
                        nodes={run.nodes || []}
                        {metrics}
                        bind:selectedNodeId
                        onNodeSelect={openNodeTerminal}
                    />
                </aside>
            {/if}

            <!-- Workspace Content Area -->
            <div
                class="min-w-0 bg-background flex flex-col relative text-foreground overflow-hidden flex-1 h-full"
            >
                <div
                    class="shrink-0 h-10 border-b flex items-center justify-between px-4 bg-muted/10 shadow-sm z-10"
                >
                    <div class="flex items-center gap-2">
                        <Button
                            variant="ghost"
                            size="icon"
                            class="h-7 w-7"
                            title={isRunSidebarOpen ? "Hide run sidebar" : "Show run sidebar"}
                            onclick={toggleRunSidebar}
                        >
                            {#if isRunSidebarOpen}
                                <PanelLeftClose class="size-4" />
                            {:else}
                                <PanelLeftOpen class="size-4" />
                            {/if}
                        </Button>
                        <div
                            class="text-sm font-medium flex items-center gap-2 text-muted-foreground"
                        >
                            Workspace
                        </div>
                    </div>
                    <div class="flex items-center gap-2">
                        <DropdownMenu.Root>
                            <DropdownMenu.Trigger>
                                {#snippet child({ props })}
                                    <Button
                                        {...props}
                                        variant="outline"
                                        size="sm"
                                        class="h-7 gap-1.5 text-xs"
                                    >
                                        <Plus class="size-3.5" />
                                        Add Panel
                                    </Button>
                                {/snippet}
                            </DropdownMenu.Trigger>
                            <DropdownMenu.Content align="end" class="w-44">
                                <DropdownMenu.Label>Add Panel</DropdownMenu.Label>
                                <DropdownMenu.Separator />
                                {#each panelOptions as option}
                                    <DropdownMenu.Item onclick={() => addWidget(option.type)}>
                                        {option.label}
                                    </DropdownMenu.Item>
                                {/each}
                            </DropdownMenu.Content>
                        </DropdownMenu.Root>
                    </div>
                </div>
                {#if showInteractionNotice}
                    <div class="shrink-0 border-b bg-sky-50 px-4 py-2 text-sm text-sky-950">
                        <div class="flex items-start justify-between gap-3">
                            <div>
                                This run is interactive. Use the
                                <span class="font-medium">Input</span> panel to send a
                                value, then watch the result appear in the
                                <span class="font-medium">Message</span> panel.
                            </div>
                            <Button
                                variant="ghost"
                                size="icon"
                                class="h-7 w-7 shrink-0 text-sky-900/70 hover:bg-sky-100 hover:text-sky-950"
                                title="Dismiss interaction hint"
                                onclick={dismissInteractionNotice}
                            >
                                <X class="size-4" />
                            </Button>
                        </div>
                    </div>
                {/if}
                <div class="flex-1 min-h-0 relative">
                    <Workspace
                        bind:layout={workspaceLayout}
                        onLayoutChange={handleLayoutChange}
                        runId={runId || ""}
                        nodes={run?.nodes || []}
                        {snapshots}
                        {inputValues}
                        onEmit={emitMessage}
                        refreshToken={messageRefreshToken}
                        {isRunActive}
                    />
                </div>
            </div>
        </div>
    {/if}
</div>
