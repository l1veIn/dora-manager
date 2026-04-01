<script lang="ts">
    import { page } from "$app/stores";
    import { onMount, onDestroy } from "svelte";
    import { get, post } from "$lib/api";
    import { goto } from "$app/navigation";
    import { Button } from "$lib/components/ui/button/index.js";

    import RunHeader from "./RunHeader.svelte";
    import RunFailureBanner from "./RunFailureBanner.svelte";
    import RunSummaryCard from "./RunSummaryCard.svelte";
    import RunNodeList from "./RunNodeList.svelte";
    import TerminalPane from "./TerminalPane.svelte";
    import InteractionPane from "./InteractionPane.svelte";

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
    let showTerminal = $state(
        typeof localStorage !== "undefined" &&
            localStorage.getItem("dm-show-terminal") !== "false",
    );
    let stoppingRun = $state(false);

    // Persist terminal toggle preference
    function setShowTerminal(value: boolean) {
        showTerminal = value;
        localStorage.setItem("dm-show-terminal", String(value));
        if (!value) selectedNodeId = "";
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
            if (run?.nodes?.length > 0 && !selectedNodeId && showTerminal) {
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

    async function fetchInteraction() {
        if (!runId) return;
        try {
            interaction = await get(`/runs/${runId}/interaction`);
        } catch (e) {
            console.error("Failed to fetch interaction state", e);
        }
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

    onMount(() => {
        fetchRunDetail();
        fetchInteraction();
        mainPolling = setInterval(() => {
            if (isRunActive) {
                fetchRunDetail();
                fetchInteraction();
            } else {
                metrics = null;
                fetchInteraction();
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
                <RunSummaryCard {run} {metrics} />
                <RunNodeList
                    nodes={run.nodes || []}
                    {metrics}
                    bind:selectedNodeId
                    onNodeSelect={(id) => {
                        selectedNodeId = id;
                        setShowTerminal(true);
                    }}
                />
            </aside>

            <!-- Content Area -->
            {#if showTerminal}
                <div class="flex-1 min-w-0 bg-background flex flex-col relative text-foreground h-full overflow-hidden">
                    {#if hasInteraction}
                        <div class="h-[45%] min-h-[260px] border-b overflow-hidden">
                            <InteractionPane
                                runId={runId || ""}
                                displays={interaction.displays}
                                inputs={interaction.inputs}
                                onEmit={emitInteraction}
                            />
                        </div>
                    {/if}
                    <div class="flex-1 min-h-0">
                        <TerminalPane
                            runId={runId || ""}
                            nodeId={selectedNodeId}
                            {isRunActive}
                            onClose={() => { setShowTerminal(false); }}
                        />
                    </div>
                </div>
            {:else}
                {#if hasInteraction}
                    <div class="flex-1 min-w-0 overflow-hidden bg-background">
                        <InteractionPane
                            runId={runId || ""}
                            displays={interaction.displays}
                            inputs={interaction.inputs}
                            onEmit={emitInteraction}
                        />
                    </div>
                {:else}
                    <div class="flex-1 min-w-0 flex items-center justify-center bg-background">
                        <p class="text-sm text-muted-foreground/60">Click a node on the left to view its logs.</p>
                    </div>
                {/if}
            {/if}
        </div>
    {/if}
</div>
