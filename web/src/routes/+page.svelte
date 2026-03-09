<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { get, post } from "$lib/api";
    import * as HoverCard from "$lib/components/ui/hover-card/index.js";
    import * as Card from "$lib/components/ui/card/index.js";
    import { useStatus } from "$lib/stores/status.svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import {
        Play,
        RefreshCw,
        CheckCircle2,
        XCircle,
        Activity,
        History,
        ArrowRight,
    } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import RecentRunCard from "$lib/components/runs/RecentRunCard.svelte";
    import { goto } from "$app/navigation";

    const store = useStatus();

    let recentRuns = $state<any[]>([]);
    let activeRuns = $state<any[]>([]);
    let frequentDataflows = $state<
        { name: string; count: number; id: string }[]
    >([]);
    let runsLoading = $state(false);
    let runsPolling: ReturnType<typeof setInterval> | null = null;

    async function fetchRunsOverview() {
        if (runsLoading) return;
        if (recentRuns.length === 0 && activeRuns.length === 0)
            runsLoading = true;
        try {
            const activeResult: any = await get(`/runs/active?metrics=true`);
            activeRuns = Array.isArray(activeResult)
                ? activeResult
                : activeResult.runs || [];

            const recentResult: any = await get(`/runs?limit=100`);
            const runs = recentResult.runs || [];

            // Calculate frequent dataflows
            const counts: Record<
                string,
                { name: string; count: number; id: string }
            > = {};
            for (const r of runs) {
                // The API /runs returns RunSummary where `name` is the dataflow id, and `id` is the Run ID.
                const df_id = r.name;
                if (df_id) {
                    if (!counts[df_id]) {
                        // make a friendly name from the dataflow id (e.g. my-flow from my-flow.yml)
                        let friendly_name =
                            df_id
                                .split("/")
                                .pop()
                                ?.replace(".yml", "")
                                ?.replace(".yaml", "") || df_id;
                        counts[df_id] = {
                            name: friendly_name,
                            count: 0,
                            id: df_id,
                        };
                    }
                    counts[df_id].count++;
                }
            }
            frequentDataflows = Object.values(counts)
                .sort((a, b) => b.count - a.count)
                .slice(0, 4);

            recentRuns = runs
                .filter((r: any) => r.status !== "running")
                .slice(0, 6);
        } catch (e) {
            console.error("Failed to fetch runs overview", e);
        } finally {
            runsLoading = false;
        }
    }

    onMount(() => {
        store.refresh();
        fetchRunsOverview();

        const onVisible = () => {
            if (document.visibilityState === "visible") {
                store.refresh();
                fetchRunsOverview();
            }
        };
        document.addEventListener("visibilitychange", onVisible);

        runsPolling = setInterval(fetchRunsOverview, 3000);

        return () => {
            document.removeEventListener("visibilitychange", onVisible);
            if (runsPolling) clearInterval(runsPolling);
        };
    });
</script>

<div
    class="p-6 max-w-7xl mx-auto space-y-8 flex flex-col min-h-[calc(10vh-4rem)]"
>
    <div class="flex items-center justify-between shrink-0">
        <div>
            <h1 class="text-3xl font-bold tracking-tight">Dashboard</h1>
            <p class="text-sm text-muted-foreground mt-1">
                Runtime health and operational overview.
            </p>
        </div>
        <div class="flex items-center gap-2">
            <Button
                variant="outline"
                size="sm"
                onclick={() => {
                    store.refresh(true);
                    fetchRunsOverview();
                }}
            >
                <RefreshCw class="mr-2 size-4" />
                Refresh
            </Button>
            <Button size="sm" onclick={() => goto("/dataflows")}>
                <Play class="mr-2 size-4" /> Start Run
            </Button>
        </div>
    </div>

    <!-- Frequent Dataflows -->
    <div class="shrink-0 space-y-4">
        <div class="flex items-center justify-between border-b pb-2">
            <h2 class="text-xl font-semibold flex items-center gap-2">
                Frequent Dataflows
            </h2>
            {#if store.doctor}
                <HoverCard.Root>
                    <HoverCard.Trigger>
                        <div
                            class="flex items-center gap-2 px-2 py-1 bg-muted/30 rounded-full text-xs border cursor-pointer hover:bg-muted/50 transition-colors"
                        >
                            <div
                                class="size-2 rounded-full {store.doctor
                                    .active_binary_ok &&
                                store.doctor.python?.found &&
                                store.doctor.uv?.found
                                    ? 'bg-green-500'
                                    : 'bg-amber-500'}"
                            ></div>
                            <span class="text-muted-foreground font-medium"
                                >System {store.doctor.active_binary_ok &&
                                store.doctor.python?.found &&
                                store.doctor.uv?.found
                                    ? "Healthy"
                                    : "Needs Attention"}</span
                            >
                        </div>
                    </HoverCard.Trigger>
                    <HoverCard.Content class="w-80">
                        <div class="space-y-2">
                            <h4 class="text-sm font-semibold">
                                Environment Health
                            </h4>
                            <p class="text-xs text-muted-foreground mb-4">
                                Diagnostic results for Dora Manager runtime.
                            </p>

                            <div
                                class="grid grid-cols-2 gap-x-4 gap-y-2 text-xs"
                            >
                                <div class="flex items-center gap-1.5">
                                    {#if store.doctor.python?.found}<CheckCircle2
                                            class="text-green-500 size-3"
                                        />{:else}<XCircle
                                            class="text-red-500 size-3"
                                        />{/if}
                                    <span
                                        class="text-muted-foreground font-medium w-12 text-right"
                                        >Python:</span
                                    >
                                    <span
                                        class="truncate"
                                        title={store.doctor.python?.path}
                                        >{store.doctor.python?.found
                                            ? "Found"
                                            : "Missing"}</span
                                    >
                                </div>
                                <div class="flex items-center gap-1.5">
                                    {#if store.doctor.uv?.found}<CheckCircle2
                                            class="text-green-500 size-3"
                                        />{:else}<XCircle
                                            class="text-red-500 size-3"
                                        />{/if}
                                    <span
                                        class="text-muted-foreground font-medium w-12 text-right"
                                        >uv:</span
                                    >
                                    <span
                                        class="truncate"
                                        title={store.doctor.uv?.path}
                                        >{store.doctor.uv?.found
                                            ? "Found"
                                            : "Missing"}</span
                                    >
                                </div>
                                <div class="flex items-center gap-1.5">
                                    {#if store.doctor.active_binary_ok}<CheckCircle2
                                            class="text-green-500 size-3"
                                        />{:else}<XCircle
                                            class="text-amber-500 size-3"
                                        />{/if}
                                    <span
                                        class="text-muted-foreground font-medium w-12 text-right"
                                        >Binary:</span
                                    >
                                    <span
                                        class="font-mono truncate"
                                        title={store.doctor.active_version}
                                        >{store.doctor.active_version ||
                                            "None"}</span
                                    >
                                </div>
                                <div class="flex items-center gap-1.5">
                                    {#if store.status?.dm_home}<CheckCircle2
                                            class="text-green-500 size-3"
                                        />{:else}<XCircle
                                            class="text-red-500 size-3"
                                        />{/if}
                                    <span
                                        class="text-muted-foreground font-medium w-12 text-right"
                                        >Home:</span
                                    >
                                    <span
                                        class="font-mono truncate"
                                        title={store.status?.dm_home}
                                        >{store.status?.dm_home ||
                                            "Missing"}</span
                                    >
                                </div>
                            </div>
                        </div>
                    </HoverCard.Content>
                </HoverCard.Root>
            {/if}
        </div>

        {#if frequentDataflows.length === 0 && !runsLoading}
            <div
                class="h-24 flex items-center justify-center border-2 border-dashed rounded-lg bg-muted/20 text-muted-foreground text-sm"
            >
                No history available to suggest frequent dataflows.
            </div>
        {:else}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                {#each frequentDataflows as fd}
                    <button
                        type="button"
                        class="text-left group flex items-start gap-4 p-4 rounded-xl border bg-card hover:bg-muted/50 transition-colors shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                        onclick={() => goto(`/dataflows/${fd.id}`)}
                    >
                        <div
                            class="h-10 w-10 shrink-0 rounded-lg bg-primary/10 flex items-center justify-center text-primary group-hover:scale-105 transition-transform"
                        >
                            <Play class="size-5 ml-0.5" />
                        </div>
                        <div class="flex flex-col min-w-0 pr-2">
                            <span class="font-medium truncate">{fd.name}</span>
                            <span class="text-xs text-muted-foreground truncate"
                                >Run {fd.count} time{fd.count === 1
                                    ? ""
                                    : "s"}</span
                            >
                        </div>
                    </button>
                {/each}
                {#if runsLoading && frequentDataflows.length === 0}
                    {#each Array(4) as _}
                        <div
                            class="h-20 rounded-xl bg-muted animate-pulse border"
                        ></div>
                    {/each}
                {/if}
            </div>
        {/if}
    </div>

    <div class="grid lg:grid-cols-5 gap-8 flex-1 min-h-[400px]">
        <!-- Active Runs -->
        <div
            class="space-y-4 flex flex-col min-h-0 lg:col-span-2 shadow-sm rounded-lg p-4 bg-muted/10 border"
        >
            <div
                class="flex items-center justify-between border-b pb-2 shrink-0"
            >
                <h2 class="text-lg font-semibold flex items-center gap-2">
                    <Activity class="size-4 text-blue-500" />
                    Active Runs
                </h2>
                <Badge variant="secondary" class="font-mono"
                    >{activeRuns.length}</Badge
                >
            </div>

            <div class="flex-1 overflow-y-auto min-h-0 pb-2">
                {#if runsLoading && activeRuns.length === 0 && recentRuns.length === 0}
                    <div
                        class="h-32 flex items-center justify-center text-sm text-muted-foreground"
                    >
                        Loading runs...
                    </div>
                {:else if activeRuns.length === 0}
                    <div
                        class="h-40 border-2 border-dashed rounded-lg flex flex-col items-center justify-center text-muted-foreground bg-muted/20"
                    >
                        <Activity class="size-8 mb-2 opacity-50" />
                        <p class="text-sm font-medium text-foreground">
                            No active runs
                        </p>
                        <p class="text-xs mt-1">Start a run from Dataflows</p>
                    </div>
                {:else}
                    <div class="flex flex-col gap-3">
                        {#each activeRuns as run}
                            <RecentRunCard {run} />
                        {/each}
                    </div>
                {/if}
            </div>
        </div>

        <!-- Recent Runs -->
        <div class="space-y-4 flex flex-col min-h-0 lg:col-span-3">
            <div
                class="flex items-center justify-between border-b pb-2 shrink-0"
            >
                <h2 class="text-lg font-semibold flex items-center gap-2">
                    <History class="size-4 text-muted-foreground" />
                    Recent Finished
                </h2>
                <Button
                    variant="ghost"
                    size="sm"
                    class="h-6 px-2 text-xs"
                    onclick={() => goto("/runs")}
                >
                    View All <ArrowRight class="size-3 ml-1" />
                </Button>
            </div>

            <div class="flex-1 overflow-y-auto min-h-0 pb-2">
                {#if runsLoading && activeRuns.length === 0 && recentRuns.length === 0}
                    <div
                        class="h-32 flex items-center justify-center text-sm text-muted-foreground"
                    >
                        Loading runs...
                    </div>
                {:else if recentRuns.length === 0}
                    <div
                        class="h-40 flex items-center justify-center text-sm text-muted-foreground"
                    >
                        No previous runs recorded.
                    </div>
                {:else}
                    <div
                        class="grid grid-cols-1 md:grid-cols-2 gap-3 lg:grid-cols-2 xl:grid-cols-2"
                    >
                        {#each recentRuns as run}
                            <div class="h-full">
                                <RecentRunCard {run} />
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>
        </div>
    </div>
</div>
