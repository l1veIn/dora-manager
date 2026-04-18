<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import * as HoverCard from "$lib/components/ui/hover-card/index.js";
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
        Sparkles,
        TimerReset,
    } from "lucide-svelte";
    import RecentRunCard from "$lib/components/runs/RecentRunCard.svelte";
    import { goto } from "$app/navigation";

    const store = useStatus();

    let recentRuns = $state<any[]>([]);
    let activeRuns = $state<any[]>([]);
    let frequentDataflows = $state<
        {
            name: string;
            count: number;
            id: string;
            destination: string;
            destinationLabel: string;
            helperText: string;
            destinationKind: "workspace" | "run" | "dataflows";
        }[]
    >([]);
    let runsLoading = $state(false);
    let runsPolling: ReturnType<typeof setInterval> | null = null;
    let hasSavedDataflows = $state(false);
    let hasUsableFrequentWorkspace = $derived(
        frequentDataflows.some((fd) => fd.destinationKind === "workspace"),
    );
    let quickStartLaunching = $state(false);
    let quickStartWorkspaceOpening = $state(false);

    const QUICKSTART_DEMO_NAME = "demo-hello-timer";
    const DASHBOARD_RECENT_FAILURE_WINDOW_HOURS = 1;
    const QUICKSTART_DEMO_YAML = `nodes:
  - id: echo
    node: dora-echo
    inputs:
      value: dora/timer/millis/1000
    outputs:
      - value

  - id: display
    node: dm-display
    inputs:
      data: echo/value
    config:
      label: "Timer Tick"
      render: text
`;
    const QUICKSTART_DEMO_VIEW = {};

    function friendlyDataflowName(dataflowId: string) {
        return (
            dataflowId
                .split("/")
                .pop()
                ?.replace(".yml", "")
                ?.replace(".yaml", "") || dataflowId
        );
    }

    function parseTimestampMs(value: string | null | undefined) {
        if (!value) return 0;
        const ms = new Date(value).getTime();
        return Number.isNaN(ms) ? 0 : ms;
    }

    function isWithinHours(value: string | null | undefined, hours: number) {
        const ms = parseTimestampMs(value);
        if (!ms) return false;
        return Date.now() - ms <= hours * 60 * 60 * 1000;
    }

    function isEphemeralRunName(name: string | null | undefined) {
        return !!name && name.startsWith(".");
    }

    function isFailureLikeRun(run: any) {
        return (
            run?.status === "failed" ||
            run?.termination_reason === "runtime_lost" ||
            run?.outcome_summary?.startsWith?.("Failed:")
        );
    }

    async function startQuickStartDemo() {
        if (quickStartLaunching) return;
        quickStartLaunching = true;
        try {
            const activeResult: any = await get(`/runs/active`);
            const activeRuns = Array.isArray(activeResult)
                ? activeResult
                : activeResult.runs || [];
            const existingRun = activeRuns.find(
                (run: any) => run.name === QUICKSTART_DEMO_NAME,
            );
            if (existingRun?.id) {
                goto(`/runs/${existingRun.id}`);
                return;
            }

            const result: any = await post("/runs/start", {
                yaml: QUICKSTART_DEMO_YAML,
                name: QUICKSTART_DEMO_NAME,
                view_json: JSON.stringify(QUICKSTART_DEMO_VIEW),
            });
            if (result.run_id) {
                goto(`/runs/${result.run_id}`);
            } else {
                goto("/runs");
            }
        } catch (e: any) {
            console.error("Failed to launch quick-start demo", e);
        } finally {
            quickStartLaunching = false;
        }
    }

    async function ensureQuickStartWorkspace() {
        try {
            await get(`/dataflows/${QUICKSTART_DEMO_NAME}`);
            return;
        } catch (e) {
            await post(`/dataflows/${QUICKSTART_DEMO_NAME}`, {
                yaml: QUICKSTART_DEMO_YAML,
            });
            await post(
                `/dataflows/${QUICKSTART_DEMO_NAME}/view`,
                QUICKSTART_DEMO_VIEW,
            );
        }
    }

    async function openQuickStartWorkspace() {
        if (quickStartWorkspaceOpening) return;
        quickStartWorkspaceOpening = true;
        try {
            await ensureQuickStartWorkspace();
            goto(`/dataflows/${QUICKSTART_DEMO_NAME}`);
        } catch (e: any) {
            console.error("Failed to open quick-start workspace", e);
        } finally {
            quickStartWorkspaceOpening = false;
        }
    }

    async function fetchRunsOverview() {
        if (runsLoading) return;
        if (recentRuns.length === 0 && activeRuns.length === 0)
            runsLoading = true;
        try {
            const [activeResult, recentResult, dataflowList] = (await Promise.all([
                get(`/runs/active?metrics=true`),
                get(`/runs?limit=100`),
                get("/dataflows").catch(() => []),
            ])) as [any, any, any[]];
            activeRuns = Array.isArray(activeResult)
                ? activeResult
                : activeResult.runs || [];

            const runs = recentResult.runs || [];
            const availableDataflows = Array.isArray(dataflowList)
                ? dataflowList
                : [];
            const availableDataflowNames = new Set(
                availableDataflows
                    .map((item: any) => item?.name)
                    .filter(Boolean),
            );
            hasSavedDataflows = availableDataflowNames.size > 0;

            // Calculate frequent dataflows
            const counts: Record<
                string,
                {
                    name: string;
                    count: number;
                    id: string;
                    latestRunId?: string;
                    latestStartedAt?: number;
                }
            > = {};
            for (const r of runs) {
                // The API /runs returns RunSummary where `name` is the dataflow id, and `id` is the Run ID.
                const df_id = r.name;
                if (df_id) {
                    if (!counts[df_id]) {
                        counts[df_id] = {
                            name: friendlyDataflowName(df_id),
                            count: 0,
                            id: df_id,
                        };
                    }
                    counts[df_id].count++;
                    const startedAt = r.started_at
                        ? new Date(r.started_at).getTime()
                        : 0;
                    if (
                        !counts[df_id].latestRunId ||
                        startedAt > (counts[df_id].latestStartedAt || 0)
                    ) {
                        counts[df_id].latestRunId = r.id;
                        counts[df_id].latestStartedAt = startedAt;
                    }
                }
            }
            frequentDataflows = Object.values(counts)
                .filter((item) => availableDataflowNames.has(item.id))
                .sort((a, b) => {
                    const aSaved = availableDataflowNames.has(a.id) ? 0 : 1;
                    const bSaved = availableDataflowNames.has(b.id) ? 0 : 1;
                    if (aSaved !== bSaved) return aSaved - bSaved;
                    if (a.count !== b.count) return b.count - a.count;
                    return (b.latestStartedAt || 0) - (a.latestStartedAt || 0);
                })
                .slice(0, 4)
                .map((item) => {
                    if (availableDataflowNames.has(item.id)) {
                        return {
                            ...item,
                            destination: `/dataflows/${item.id}`,
                            destinationLabel: "Open workspace",
                            helperText: `Saved workspace · run ${item.count} time${item.count === 1 ? "" : "s"}`,
                            destinationKind: "workspace" as const,
                        };
                    }

                    if (item.latestRunId) {
                        return {
                            ...item,
                            destination: `/runs/${item.latestRunId}`,
                            destinationLabel: "View latest run",
                            helperText: "Workspace missing · opening the latest recorded run instead",
                            destinationKind: "run" as const,
                        };
                    }

                    return {
                        ...item,
                        destination: "/dataflows",
                        destinationLabel: "Browse dataflows",
                        helperText: "No saved workspace found yet",
                        destinationKind: "dataflows" as const,
                    };
                });

            recentRuns = runs
                .filter((r: any) => r.status !== "running")
                .filter((r: any) => !isEphemeralRunName(r.name))
                .filter(
                    (r: any) =>
                        !isFailureLikeRun(r) ||
                        isWithinHours(
                            r.finished_at || r.started_at,
                            DASHBOARD_RECENT_FAILURE_WINDOW_HOURS,
                        ),
                )
                .sort((a: any, b: any) => {
                    const aSaved = availableDataflowNames.has(a.name) ? 0 : 1;
                    const bSaved = availableDataflowNames.has(b.name) ? 0 : 1;
                    if (aSaved !== bSaved) return aSaved - bSaved;
                    const aFailed = isFailureLikeRun(a) ? 1 : 0;
                    const bFailed = isFailureLikeRun(b) ? 1 : 0;
                    if (aFailed !== bFailed) return aFailed - bFailed;
                    return (
                        parseTimestampMs(b.finished_at || b.started_at) -
                        parseTimestampMs(a.finished_at || a.started_at)
                    );
                })
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

    <section class="shrink-0 space-y-4">
        <div class="flex items-center justify-between border-b pb-2">
            <div>
                <h2 class="text-xl font-semibold flex items-center gap-2">
                    <Sparkles class="size-5 text-amber-500" />
                    Quick Start
                </h2>
                <p class="text-sm text-muted-foreground mt-1">
                    Use a known-good built-in demo to reach your first successful run.
                </p>
            </div>
        </div>

        <div
            class="rounded-2xl border bg-gradient-to-br from-amber-50 via-background to-background p-5 shadow-sm dark:from-amber-950/20"
        >
            <div class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                <div class="space-y-2 max-w-2xl">
                    <div class="flex items-center gap-2 text-sm font-medium text-foreground">
                        <TimerReset class="size-4 text-amber-600" />
                        Hello Timer Demo
                    </div>
                    <p class="text-sm text-muted-foreground">
                        The fastest path to first success. This demo uses only built-in nodes,
                        starts a ticking flow immediately, and now also gives you a matching
                        editable workspace for your first small change.
                    </p>
                    <p class="text-xs text-muted-foreground">
                        Path: Dashboard -> run demo -> inspect live status -> open editable
                        workspace -> rerun.
                    </p>
                </div>
                <div class="flex items-center gap-2">
                    <Button
                        size="sm"
                        onclick={startQuickStartDemo}
                        disabled={quickStartLaunching}
                    >
                        <Play class="mr-2 size-4" />
                        {quickStartLaunching ? "Launching..." : "Run Hello Timer"}
                    </Button>
                    <Button
                        variant="outline"
                        size="sm"
                        onclick={openQuickStartWorkspace}
                        disabled={quickStartWorkspaceOpening}
                    >
                        {quickStartWorkspaceOpening
                            ? "Opening workspace..."
                            : "Edit Hello Timer"}
                    </Button>
                </div>
            </div>
        </div>
    </section>

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
                class="border-2 border-dashed rounded-lg bg-muted/20 p-5 text-sm"
            >
                <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                    <div class="space-y-1">
                        <p class="font-medium text-foreground">
                            No frequent dataflows yet
                        </p>
                        <p class="text-muted-foreground">
                            Start from a saved dataflow or browse recent runs.
                            This area will learn from the runs you keep using.
                        </p>
                    </div>
                    <div class="flex items-center gap-2">
                        <Button
                            variant="outline"
                            size="sm"
                            onclick={() => goto("/runs")}
                        >
                            View Runs
                        </Button>
                        <Button size="sm" onclick={() => goto("/dataflows")}>
                            Open Dataflows
                        </Button>
                    </div>
                </div>
            </div>
        {:else}
            {#if !hasUsableFrequentWorkspace}
                <div
                    class="rounded-lg border bg-muted/20 p-3 text-sm text-muted-foreground"
                >
                    These suggestions come from run history. If a saved
                    workspace no longer exists, the shortcut opens the latest
                    matching run so you can still inspect what happened.
                </div>
            {/if}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                {#each frequentDataflows as fd}
                    <button
                        type="button"
                        class="text-left group flex items-start gap-4 p-4 rounded-xl border bg-card hover:bg-muted/50 transition-colors shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                        onclick={() => goto(fd.destination)}
                    >
                        <div
                            class="h-10 w-10 shrink-0 rounded-lg bg-primary/10 flex items-center justify-center text-primary group-hover:scale-105 transition-transform"
                        >
                            <Play class="size-5 ml-0.5" />
                        </div>
                        <div class="flex flex-col min-w-0 pr-2">
                            <span class="font-medium truncate">{fd.name}</span>
                            <span
                                class="text-xs text-muted-foreground truncate"
                                >{fd.helperText}</span
                            >
                            <span class="text-xs text-foreground/80 mt-1">
                                {fd.destinationLabel}
                            </span>
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
                <div class="space-y-1">
                    <h2 class="text-lg font-semibold flex items-center gap-2">
                        <History class="size-4 text-muted-foreground" />
                        Recent Finished Runs
                    </h2>
                    <p class="text-xs text-muted-foreground">
                        Showing the latest completed runs from local history.
                        Use View All to inspect or clean up older entries.
                    </p>
                </div>
                <div class="flex items-center gap-2">
                    {#if !hasSavedDataflows}
                        <Button
                            variant="outline"
                            size="sm"
                            class="h-6 px-2 text-xs"
                            onclick={() => goto("/dataflows")}
                        >
                            Create One
                        </Button>
                    {/if}
                    <Button
                        variant="ghost"
                        size="sm"
                        class="h-6 px-2 text-xs"
                        onclick={() => goto("/runs")}
                    >
                        View All <ArrowRight class="size-3 ml-1" />
                    </Button>
                </div>
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
