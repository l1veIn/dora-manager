<script lang="ts">
    import { page } from "$app/stores";
    import { onMount, onDestroy } from "svelte";
    import { browser } from "$app/environment";
    import { get, post } from "$lib/api";
    import { goto } from "$app/navigation";
    import { Button } from "$lib/components/ui/button/index.js";

    import RunHeader from "./RunHeader.svelte";
    import RunFailureBanner from "./RunFailureBanner.svelte";
    import RunSummaryCard from "./RunSummaryCard.svelte";
    import RunNodeList from "./RunNodeList.svelte";
    
    import Workspace from "$lib/components/workspace/Workspace.svelte";
    import { type WorkspaceGridItem, getDefaultLayout, mutateTreeInjectTerminal, generateId } from "$lib/components/workspace/types";

    let runId = $derived($page.params.id);

    let run = $state<any>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let metrics = $state<any>(null);
    let interaction = $state<{ displays: any[]; inputs: any[] }>({
        displays: [],
        inputs: [],
    });

    let selectedNodeId = $state<string>("");
    let workspaceLayout = $state<WorkspaceGridItem[]>(getDefaultLayout());
    let workspaceLoaded = false;
    let stoppingRun = $state(false);
    let interactionSocket: WebSocket | null = null;
    let reconnectInteractionSocket: ReturnType<typeof setTimeout> | null = null;
    let interactionRefreshInFlight: Promise<void> | null = null;

    // Layout persistence
    function handleLayoutChange(newLayout: WorkspaceGridItem[]) {
        workspaceLayout = newLayout;
        if (run?.name && browser) {
            localStorage.setItem(`dm-workspace-layout-${run.name}`, JSON.stringify(newLayout));
        }
    }

    function addWidget(type: "stream" | "input" | "terminal") {
        let maxY = 0;
        for (let item of workspaceLayout) {
            maxY = Math.max(maxY, item.y + item.h);
        }
        workspaceLayout = [
            ...workspaceLayout,
            {
                id: generateId(),
                widgetType: type,
                config: {},
                x: 0, y: maxY, w: 6, h: 4
            }
        ];
        handleLayoutChange(workspaceLayout);
    }

    function openNodeTerminal(id: string) {
        selectedNodeId = id;
        
        // Find existing terminal for this node
        let targetTx = workspaceLayout.find(item => item.widgetType === "terminal" && item.config?.nodeId === id);
        
        if (!targetTx) {
            // Find any existing terminal to recycle
            let anyTx = workspaceLayout.find(item => item.widgetType === "terminal");
            if (anyTx) {
                if (!anyTx.config) anyTx.config = {};
                anyTx.config.nodeId = id;
                targetTx = anyTx;
                handleLayoutChange(workspaceLayout);
            } else {
                // Append a new terminal
                workspaceLayout = mutateTreeInjectTerminal(workspaceLayout, id);
                handleLayoutChange(workspaceLayout);
                targetTx = workspaceLayout.find(item => item.widgetType === "terminal" && item.config?.nodeId === id);
            }
        }
        
        if (targetTx) {
            const txId = targetTx.id;
            setTimeout(() => {
                const el = document.querySelector(`[gs-id="${txId}"]`) as HTMLElement;
                if (el) {
                    // Focus & animate frame jump
                    el.scrollIntoView({ behavior: 'smooth', block: 'center' });
                    const wrapper = el.querySelector('.grid-stack-item-content > div') as HTMLElement;
                    if (wrapper) {
                        wrapper.classList.remove('ring-offset-2', 'ring-2', 'ring-primary/80');
                        // forced reflow trick
                        void wrapper.offsetWidth;
                        wrapper.classList.add('transition-all', 'duration-500', 'ring-offset-2', 'ring-2', 'ring-primary/80');
                        setTimeout(() => wrapper.classList.remove('ring-offset-2', 'ring-2', 'ring-primary/80'), 1500);
                    }
                }
            }, 100); // slight delay to allow Svelte DOM flush for new components
        }
    }

    let isRunActive = $derived(run?.status === "running");
    let hasInteraction = $derived(
        (interaction?.displays?.length ?? 0) > 0 ||
            (interaction?.inputs?.length ?? 0) > 0,
    );

    // ── Data fetching ──

    async function fetchRunDetail() {
        if (!runId) return;
        try {
            const result = await get(
                `/runs/${runId}${isRunActive || loading ? "?include_metrics=true" : ""}`,
            );
            run = result;
            metrics = (result as any)?.metrics ?? null;
            if (run?.nodes?.length > 0 && !workspaceLoaded) {
                // Restore layout on first run load
                workspaceLoaded = true;
                const saved = localStorage.getItem(`dm-workspace-layout-${run.name}`);
                if (saved) {
                    try { 
                        const parsed = JSON.parse(saved); 
                        if (Array.isArray(parsed)) {
                            workspaceLayout = parsed;
                        } else {
                            console.warn("Discarding old Workspace layout version from LocalStorage");
                        }
                    } catch (e) {}
                }
            }
        } catch (e: any) {
            console.error("Failed to fetch run", e);
            error = e.message || "Run not found";
        } finally {
            loading = false;
        }
    }

    async function fetchInteraction() {
        if (!runId) return;
        if (interactionRefreshInFlight) {
            return interactionRefreshInFlight;
        }

        interactionRefreshInFlight = (async () => {
            try {
                interaction = await get(`/runs/${runId}/interaction`);
            } catch (e) {
                console.error("Failed to fetch interaction state", e);
            } finally {
                interactionRefreshInFlight = null;
            }
        })();

        return interactionRefreshInFlight;
    }

    async function emitInteraction(nodeId: string, outputId: string, value: any) {
        if (!runId) return;
        await post(`/runs/${runId}/interaction/input/events`, {
            node_id: nodeId,
            output_id: outputId,
            value,
        });
        await fetchInteraction();
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

    // ── Lifecycle ──

    let mainPolling: ReturnType<typeof setInterval> | null = null;

    function scheduleInteractionSocketReconnect() {
        if (!browser || reconnectInteractionSocket || !runId) return;
        reconnectInteractionSocket = setTimeout(() => {
            reconnectInteractionSocket = null;
            connectInteractionSocket();
        }, 1000);
    }

    function closeInteractionSocket() {
        if (reconnectInteractionSocket) {
            clearTimeout(reconnectInteractionSocket);
            reconnectInteractionSocket = null;
        }
        if (interactionSocket) {
            interactionSocket.onopen = null;
            interactionSocket.onmessage = null;
            interactionSocket.onerror = null;
            interactionSocket.onclose = null;
            interactionSocket.close();
            interactionSocket = null;
        }
    }

    function connectInteractionSocket() {
        if (!browser || !runId) return;

        closeInteractionSocket();

        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const socket = new WebSocket(
            `${protocol}//${window.location.host}/api/runs/${runId}/interaction/ws`,
        );

        socket.onmessage = async () => {
            await fetchInteraction();
        };
        socket.onerror = () => {
            socket.close();
        };
        socket.onclose = () => {
            if (interactionSocket === socket) {
                interactionSocket = null;
            }
            scheduleInteractionSocketReconnect();
        };

        interactionSocket = socket;
    }

    onMount(() => {
        fetchRunDetail();
        fetchInteraction();
        connectInteractionSocket();
        mainPolling = setInterval(() => {
            if (isRunActive) {
                fetchRunDetail();
            } else {
                metrics = null;
            }
        }, 3000);
    });

    onDestroy(() => {
        if (mainPolling) clearInterval(mainPolling);
        closeInteractionSocket();
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
                <RunSummaryCard {run} {metrics} />
                <RunNodeList
                    nodes={run.nodes || []}
                    {metrics}
                    bind:selectedNodeId
                    onNodeSelect={openNodeTerminal}
                />
            </aside>

            <!-- Workspace Content Area -->
            <div class="flex-1 min-w-0 bg-background flex flex-col relative text-foreground h-full overflow-hidden">
                <div class="shrink-0 h-10 border-b flex items-center justify-between px-4 bg-muted/10 shadow-sm z-10">
                    <div class="text-sm font-medium flex items-center gap-2 text-muted-foreground">
                        Dashboard
                    </div>
                    <div class="flex items-center gap-2">
                        <Button variant="outline" size="sm" class="h-7 text-xs" onclick={() => addWidget("stream")}>⊕ Stream</Button>
                        <Button variant="outline" size="sm" class="h-7 text-xs" onclick={() => addWidget("input")}>⊕ Input</Button>
                        <Button variant="outline" size="sm" class="h-7 text-xs" onclick={() => addWidget("terminal")}>⊕ Terminal</Button>
                    </div>
                </div>
                <div class="flex-1 min-h-0 relative">
                    <Workspace 
                        bind:layout={workspaceLayout}
                        onLayoutChange={handleLayoutChange}
                        runId={runId || ""} 
                        nodes={run?.nodes || []}
                        displays={interaction.displays} 
                        inputs={interaction.inputs}
                        onEmit={emitInteraction}
                    />
                </div>
            </div>
        </div>
    {/if}
</div>
