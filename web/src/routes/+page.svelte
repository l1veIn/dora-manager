<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import * as Card from "$lib/components/ui/card/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import {
        Play,
        Square,
        RefreshCw,
        CheckCircle2,
        XCircle,
    } from "lucide-svelte";

    let status = $state<any>(null);
    let versions = $state<any>(null);
    let doctor = $state<any>(null);
    let nodes = $state<any[]>([]);
    let loading = $state(true);

    async function refreshData() {
        try {
            [status, versions, doctor, nodes] = await Promise.all([
                get("/status").catch(() => null),
                get("/versions").catch(() => null),
                get("/doctor").catch(() => null),
                get("/nodes").catch(() => []),
            ] as any[]);
        } finally {
            loading = false;
        }
    }

    async function toggleStatus() {
        if (!status) return;
        const isRunning = status.dora_daemon_status === "running";
        try {
            if (isRunning) {
                await post("/down");
            } else {
                await post("/up");
            }
            setTimeout(refreshData, 1000); // Give it a second to apply
        } catch (e) {
            console.error("Failed to toggle status", e);
        }
    }

    onMount(() => {
        refreshData();
        const interval = setInterval(refreshData, 10000);
        return () => clearInterval(interval);
    });
</script>

<div class="p-6 max-w-6xl mx-auto space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold tracking-tight">Dashboard</h1>
        <Button variant="outline" size="sm" onclick={refreshData}>
            <RefreshCw class="mr-2 size-4" />
            Refresh
        </Button>
    </div>

    {#if loading && !status}
        <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {#each Array(3) as _}
                <Card.Root class="animate-pulse h-48 bg-muted/50"></Card.Root>
            {/each}
        </div>
    {:else}
        <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            <!-- Status Card -->
            <Card.Root>
                <Card.Header
                    class="pb-2 flex flex-row items-center justify-between space-y-0"
                >
                    <Card.Title class="text-lg font-medium"
                        >Dora Status</Card.Title
                    >
                    <div
                        class="h-3 w-3 rounded-full {status?.dora_daemon_status ===
                        'running'
                            ? 'bg-green-500'
                            : 'bg-slate-400'}"
                    ></div>
                </Card.Header>
                <Card.Content>
                    {#if status}
                        <div class="space-y-4">
                            <div class="flex flex-col gap-1">
                                <span class="text-2xl font-bold">
                                    {status.dora_daemon_status === "running"
                                        ? "Running"
                                        : "Stopped"}
                                </span>
                                <span
                                    class="text-sm text-muted-foreground whitespace-pre-wrap font-mono mt-2 bg-muted/30 p-2 rounded-md border text-xs"
                                    >{status.dora_coordinator_status ||
                                        "Unknown Coordinator"}</span
                                >
                            </div>
                            <Button
                                variant={status.dora_daemon_status === "running"
                                    ? "destructive"
                                    : "default"}
                                class="w-full"
                                onclick={toggleStatus}
                            >
                                {#if status.dora_daemon_status === "running"}
                                    <Square class="mr-2 size-4" /> Stop Dora
                                {:else}
                                    <Play class="mr-2 size-4" /> Start Dora
                                {/if}
                            </Button>
                        </div>
                    {:else}
                        <p class="text-sm text-muted-foreground">
                            Unable to fetch status.
                        </p>
                    {/if}
                </Card.Content>
            </Card.Root>

            <!-- Versions Card -->
            <Card.Root>
                <Card.Header class="pb-2">
                    <Card.Title class="text-lg font-medium">Versions</Card.Title
                    >
                </Card.Header>
                <Card.Content>
                    {#if versions}
                        <div class="space-y-4">
                            <div class="flex items-center justify-between">
                                <span class="text-sm font-medium">Active:</span>
                                <Badge variant="default" class="font-mono"
                                    >{versions.installed?.find(
                                        (v: any) => v.active,
                                    )?.version || "None"}</Badge
                                >
                            </div>
                            <div class="flex items-center justify-between">
                                <span class="text-sm font-medium"
                                    >Installed:</span
                                >
                                <span class="text-sm text-muted-foreground"
                                    >{versions.installed?.length || 0}</span
                                >
                            </div>
                            {#if versions.installed && versions.installed.length > 0}
                                <div class="pt-2 border-t">
                                    <div
                                        class="text-xs text-muted-foreground mb-2"
                                    >
                                        Installed Versions:
                                    </div>
                                    <div class="flex flex-wrap gap-1">
                                        {#each versions.installed as v}
                                            <Badge
                                                variant={v.active
                                                    ? "default"
                                                    : "secondary"}
                                                class="font-mono text-[10px]"
                                                >{v.version}</Badge
                                            >
                                        {/each}
                                    </div>
                                </div>
                            {/if}
                        </div>
                    {:else}
                        <p class="text-sm text-muted-foreground">
                            Unable to fetch versions.
                        </p>
                    {/if}
                </Card.Content>
            </Card.Root>

            <!-- Quick Stats Card -->
            <Card.Root>
                <Card.Header class="pb-2">
                    <Card.Title class="text-lg font-medium"
                        >Quick Stats</Card.Title
                    >
                </Card.Header>
                <Card.Content>
                    <div class="space-y-4">
                        <div class="flex items-center justify-between">
                            <span class="text-sm font-medium"
                                >Installed Nodes:</span
                            >
                            <span class="text-2xl font-bold"
                                >{nodes?.length || 0}</span
                            >
                        </div>
                        <!-- More stats can be added here easily -->
                    </div>
                </Card.Content>
            </Card.Root>

            <!-- Health Card (Wide) -->
            <Card.Root class="md:col-span-2 lg:col-span-3">
                <Card.Header class="pb-2">
                    <Card.Title class="text-lg font-medium"
                        >Environment Health</Card.Title
                    >
                </Card.Header>
                <Card.Content>
                    {#if doctor}
                        <ul
                            class="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm"
                        >
                            <li class="flex items-center gap-2">
                                {#if doctor.python?.found}
                                    <CheckCircle2
                                        class="text-green-500 size-4"
                                    />
                                {:else}
                                    <XCircle class="text-red-500 size-4" />
                                {/if}
                                <span class="font-medium">Python 3:</span>
                                <span class="text-muted-foreground ml-auto"
                                    >{doctor.python?.path || "Missing"}</span
                                >
                            </li>
                            <li class="flex items-center gap-2">
                                {#if doctor.uv?.found}
                                    <CheckCircle2
                                        class="text-green-500 size-4"
                                    />
                                {:else}
                                    <XCircle class="text-red-500 size-4" />
                                {/if}
                                <span class="font-medium">uv:</span>
                                <span class="text-muted-foreground ml-auto"
                                    >{doctor.uv?.path || "Missing"}</span
                                >
                            </li>
                            <li class="flex items-center gap-2">
                                {#if doctor.active_binary_ok}
                                    <CheckCircle2
                                        class="text-green-500 size-4"
                                    />
                                {:else}
                                    <XCircle class="text-amber-500 size-4" />
                                {/if}
                                <span class="font-medium"
                                    >Active Dora Binary:</span
                                >
                                <span
                                    class="text-muted-foreground ml-auto font-mono text-xs"
                                    >{doctor.active_version || "None"}</span
                                >
                            </li>
                            <li class="flex items-center gap-2">
                                {#if status?.dm_home}
                                    <CheckCircle2
                                        class="text-green-500 size-4"
                                    />
                                {:else}
                                    <XCircle class="text-red-500 size-4" />
                                {/if}
                                <span class="font-medium">DM Home:</span>
                                <span
                                    class="text-muted-foreground ml-auto font-mono text-xs"
                                    >{status?.dm_home || "Missing"}</span
                                >
                            </li>
                        </ul>
                    {:else}
                        <p class="text-sm text-muted-foreground">
                            Unable to fetch health status.
                        </p>
                    {/if}
                </Card.Content>
            </Card.Root>
        </div>
    {/if}
</div>
