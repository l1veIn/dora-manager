<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { get, post } from "$lib/api";
    import { useStatus } from "$lib/stores/status.svelte";
    import * as Card from "$lib/components/ui/card/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import {
        Play,
        Square,
        RefreshCw,
        CheckCircle2,
        XCircle,
        Loader2,
        Activity,
        History,
        ArrowRight,
    } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import RecentRunCard from "$lib/components/runs/RecentRunCard.svelte";
    import { goto } from "$app/navigation";

    const store = useStatus();
    let toggling = $state(false);

    let recentRuns = $state<any[]>([]);
    let activeRuns = $state<any[]>([]);
    let runsLoading = $state(false);
    let runsPolling: ReturnType<typeof setInterval> | null = null;

    async function toggleStatus() {
        if (!store.status || toggling) return;
        toggling = true;
        const isRunning = store.status.runtime_running;
        const action = isRunning ? "Stop" : "Start";
        try {
            const res: any = isRunning
                ? await post("/down")
                : await post("/up");
            if (res.success) {
                toast.success(`${action} succeeded`);
            } else {
                toast.error(`${action} failed: ${res.message}`);
            }
        } catch (e: any) {
            toast.error(`${action} failed: ${e.message}`);
        } finally {
            await store.refresh();
            toggling = false;
        }
    }

    async function fetchRunsOverview() {
        if (runsLoading) return;
        if (recentRuns.length === 0 && activeRuns.length === 0)
            runsLoading = true;
        try {
            const activeResult: any = await get(`/runs/active`);
            activeRuns = Array.isArray(activeResult)
                ? activeResult
                : activeResult.runs || [];

            const recentResult: any = await get(`/runs?limit=15`);
            const runs = recentResult.runs || [];

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
    class="p-6 max-w-7xl mx-auto space-y-8 flex flex-col min-h-[calc(100vh-4rem)]"
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

    <!-- Runtime Health Summary -->
    <div class="shrink-0 space-y-4">
        <h2 class="text-xl font-semibold flex items-center gap-2 border-b pb-2">
            Runtime Health
        </h2>

        {#if store.loading && !store.status}
            <div class="grid gap-4 md:grid-cols-4">
                {#each Array(4) as _}
                    <div
                        class="h-32 rounded-lg bg-muted animate-pulse border"
                    ></div>
                {/each}
            </div>
        {:else}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                <!-- Status Card -->
                <Card.Root class="flex flex-col">
                    <Card.Header
                        class="pb-2 pt-4 px-4 flex flex-row items-center justify-between space-y-0 text-muted-foreground"
                    >
                        <span class="text-sm font-medium text-foreground"
                            >Dora Status</span
                        >
                        <div
                            class="h-2.5 w-2.5 rounded-full {toggling
                                ? 'bg-amber-400 animate-pulse'
                                : store.status?.runtime_running
                                  ? 'bg-green-500'
                                  : 'bg-slate-400'}"
                        ></div>
                    </Card.Header>
                    <Card.Content
                        class="px-4 pb-4 flex-1 flex flex-col justify-between"
                    >
                        {#if store.status}
                            <div class="flex flex-col gap-1 mb-4">
                                <span class="text-xl font-bold">
                                    {store.status.runtime_running
                                        ? "Running"
                                        : "Stopped"}
                                </span>
                            </div>
                            <Button
                                variant={store.status.runtime_running
                                    ? "secondary"
                                    : "default"}
                                size="sm"
                                class="w-full text-xs"
                                onclick={toggleStatus}
                                disabled={toggling}
                            >
                                {#if toggling}
                                    <Loader2
                                        class="mr-1.5 size-3.5 animate-spin"
                                    />
                                    {store.status.runtime_running
                                        ? "Stopping..."
                                        : "Starting..."}
                                {:else if store.status.runtime_running}
                                    <Square class="mr-1.5 size-3.5" /> Stop Dora
                                {:else}
                                    <Play class="mr-1.5 size-3.5" /> Start Dora
                                {/if}
                            </Button>
                        {:else}
                            <p class="text-xs text-muted-foreground my-auto">
                                Unable to fetch status.
                            </p>
                        {/if}
                    </Card.Content>
                </Card.Root>

                <!-- Health Check Card -->
                <Card.Root class="lg:col-span-2 flex flex-col">
                    <Card.Header class="pb-2 pt-4 px-4 text-sm font-medium"
                        >Environment Health</Card.Header
                    >
                    <Card.Content
                        class="px-4 pb-4 flex-1 flex flex-col justify-end"
                    >
                        {#if store.doctor}
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
                        {:else}
                            <p class="text-xs text-muted-foreground">
                                Unable to fetch health status.
                            </p>
                        {/if}
                    </Card.Content>
                </Card.Root>

                <!-- Quick Stats / Versions -->
                <Card.Root class="flex flex-col">
                    <Card.Header
                        class="pb-2 pt-4 px-4 text-sm font-medium flex justify-between flex-row items-center"
                    >
                        Quick Stats
                    </Card.Header>
                    <Card.Content
                        class="px-4 pb-4 flex-1 flex flex-col justify-end"
                    >
                        <div class="space-y-3">
                            <div
                                class="flex items-center justify-between border-b pb-2"
                            >
                                <span class="text-xs text-muted-foreground"
                                    >Installed Nodes</span
                                >
                                <span class="text-sm font-mono font-semibold"
                                    >{store.nodes?.length || 0}</span
                                >
                            </div>
                            <div class="flex items-center justify-between">
                                <span class="text-xs text-muted-foreground"
                                    >Dora Versions</span
                                >
                                <span class="text-sm font-mono"
                                    >{store.doctor?.installed_versions
                                        ?.length || 0}</span
                                >
                            </div>
                        </div>
                    </Card.Content>
                </Card.Root>
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
