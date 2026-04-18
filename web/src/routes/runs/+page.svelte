<script lang="ts">
    import { onMount, tick } from "svelte";
    import { get, del, post } from "$lib/api";
    import * as Table from "$lib/components/ui/table/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import {
        RefreshCw,
        ChevronLeft,
        ChevronRight,
        Search,
        LayoutDashboard,
        Trash2,
    } from "lucide-svelte";
    import RunStatusBadge from "$lib/components/runs/RunStatusBadge.svelte";
    import { summarizeOutcomeSummary } from "$lib/runs/outcomeSummary";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Checkbox } from "$lib/components/ui/checkbox/index.js";
    import { goto } from "$app/navigation";

    let runs = $state<any[]>([]);
    let loading = $state(true);
    let totalRuns = $state(0);
    let selectedRunIds = $state<string[]>([]);
    let isDeleting = $state(false);
    let isDeleteDialogOpen = $state(false);

    // Filters & Pagination
    let currentPage = $state(1);
    let pageSize = $state(20);
    let statusFilter = $state("all"); // 'all', 'running', 'succeeded', 'stopped', 'failed'
    let searchQuery = $state(""); // search by dataflow_name or run_id

    // Selected options for Select components
    let pageSizeStr = $state("20");

    // Sync options to state
    $effect(() => {
        let newSize = parseInt(pageSizeStr, 10);
        if (pageSize !== newSize) {
            pageSize = newSize;
            currentPage = 1;
            fetchRuns();
        }
    });

    $effect(() => {
        if (statusFilter) {
            currentPage = 1;
            fetchRuns();
        }
    });

    let debounceTimer: ReturnType<typeof setTimeout>;
    function handleSearchInput() {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
            currentPage = 1;
            fetchRuns();
        }, 300);
    }

    async function fetchRuns() {
        loading = true;
        try {
            const params = new URLSearchParams();
            params.append("limit", pageSize.toString());
            params.append("offset", ((currentPage - 1) * pageSize).toString());
            if (statusFilter !== "all") {
                params.append("status", statusFilter);
            }
            if (searchQuery.trim() !== "") {
                params.append("search", searchQuery.trim());
            }

            const result = (await get(`/runs?${params.toString()}`)) as any;
            runs = result.runs || [];
            totalRuns = result.total || 0;
        } catch (e) {
            console.error("Failed to fetch runs", e);
        } finally {
            loading = false;
        }
    }

    let isAllSelected = $derived(
        runs.length > 0 && selectedRunIds.length === runs.length,
    );

    function toggleAll(checked: boolean) {
        if (checked) {
            selectedRunIds = runs.map((r) => r.id);
        } else {
            selectedRunIds = [];
        }
    }

    function toggleSelection(runId: string, checked: boolean) {
        if (checked) {
            if (!selectedRunIds.includes(runId)) {
                selectedRunIds = [...selectedRunIds, runId];
            }
        } else {
            selectedRunIds = selectedRunIds.filter((id) => id !== runId);
        }
    }

    function openDeleteDialog() {
        if (selectedRunIds.length === 0) return;
        isDeleteDialogOpen = true;
    }

    async function confirmDeleteSelectedRuns() {
        isDeleteDialogOpen = false;
        isDeleting = true;
        try {
            await post("/runs/delete", { run_ids: selectedRunIds });
            selectedRunIds = [];
            fetchRuns();
        } catch (e: any) {
            console.error("Failed to delete runs", e);
            alert("Failed to delete runs: " + e.message);
        } finally {
            isDeleting = false;
        }
    }

    function formatTime(ts: string) {
        if (!ts) return "-";
        return new Date(ts).toLocaleString();
    }

    function rowClick(runId: string) {
        goto(`/runs/${runId}`);
    }

    onMount(() => {
        fetchRuns();
    });
</script>

<div class="p-6 max-w-7xl mx-auto flex flex-col h-full gap-4">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-3xl font-bold tracking-tight">Runs</h1>
            <p class="text-sm text-muted-foreground mt-1">
                Dataflow execution history and instances.
            </p>
        </div>

        <div class="flex items-center gap-2">
            {#if selectedRunIds.length > 0}
                <Button
                    variant="destructive"
                    size="sm"
                    onclick={openDeleteDialog}
                    disabled={isDeleting}
                >
                    <Trash2
                        class="mr-2 size-4 {isDeleting ? 'animate-pulse' : ''}"
                    />
                    {isDeleting
                        ? "Deleting..."
                        : `Delete Selected (${selectedRunIds.length})`}
                </Button>
            {/if}
            <Button variant="outline" size="sm" onclick={fetchRuns}>
                <RefreshCw
                    class="mr-2 size-4 {loading ? 'animate-spin' : ''}"
                /> Refresh
            </Button>
        </div>
    </div>

    <!-- Filters/Search/Pagination Controls -->
    <div
        class="flex flex-col sm:flex-row gap-3 items-center justify-between border bg-card p-3 rounded-md shadow-sm shrink-0"
    >
        <div class="flex items-center gap-3 w-full sm:w-auto flex-1">
            <div class="relative max-w-sm w-full">
                <Search
                    class="absolute left-2.5 top-2.5 size-4 text-muted-foreground"
                />
                <Input
                    type="text"
                    placeholder="Search by name or run ID..."
                    class="pl-9 bg-background w-full"
                    bind:value={searchQuery}
                    oninput={handleSearchInput}
                />
            </div>

            <div class="w-[140px]">
                <Select.Root type="single" bind:value={statusFilter}>
                    <Select.Trigger class="bg-background">
                        {statusFilter === "all" ? "All Status" : statusFilter}
                    </Select.Trigger>
                    <Select.Content>
                        <Select.Item value="all">All Status</Select.Item>
                        <Select.Item value="running">Running</Select.Item>
                        <Select.Item value="succeeded">Succeeded</Select.Item>
                        <Select.Item value="stopped">Stopped</Select.Item>
                        <Select.Item value="failed">Failed</Select.Item>
                    </Select.Content>
                </Select.Root>
            </div>
        </div>

        <div class="flex items-center gap-3 shrink-0">
            <div class="w-[120px]">
                <Select.Root type="single" bind:value={pageSizeStr}>
                    <Select.Trigger class="bg-background">
                        {pageSizeStr} / page
                    </Select.Trigger>
                    <Select.Content>
                        <Select.Item value="10">10 / page</Select.Item>
                        <Select.Item value="20">20 / page</Select.Item>
                        <Select.Item value="50">50 / page</Select.Item>
                        <Select.Item value="100">100 / page</Select.Item>
                    </Select.Content>
                </Select.Root>
            </div>

            <div
                class="flex items-center gap-1 border rounded-md overflow-hidden bg-background"
            >
                <Button
                    variant="ghost"
                    size="icon"
                    class="rounded-none h-9 w-9"
                    disabled={currentPage <= 1 || loading}
                    onclick={() => {
                        currentPage--;
                        fetchRuns();
                    }}
                >
                    <ChevronLeft class="size-4" />
                </Button>
                <div
                    class="text-sm font-medium w-12 text-center select-none text-muted-foreground"
                >
                    {currentPage}
                </div>
                <Button
                    variant="ghost"
                    size="icon"
                    class="rounded-none h-9 w-9"
                    disabled={currentPage * pageSize >= totalRuns || loading}
                    onclick={() => {
                        currentPage++;
                        fetchRuns();
                    }}
                >
                    <ChevronRight class="size-4" />
                </Button>
            </div>
        </div>
    </div>

    <!-- Runs Table -->
    <div
        class="border rounded-md shrink-0 overflow-auto bg-card flex-1 shadow-sm"
    >
        <Table.Root>
            <Table.Header class="sticky top-0 bg-card z-10 shadow-sm">
                <Table.Row>
                    <Table.Head class="w-[40px] text-center">
                        <Checkbox
                            checked={isAllSelected}
                            onCheckedChange={(v: any) => toggleAll(v === true)}
                            class="translate-y-[2px]"
                        />
                    </Table.Head>
                    <Table.Head class="w-[200px]">Name</Table.Head>
                    <Table.Head class="w-[100px]">Status</Table.Head>
                    <Table.Head class="w-[160px]">Started</Table.Head>
                    <Table.Head class="w-[160px]">Finished</Table.Head>
                    <Table.Head class="w-[80px] text-center">Nodes</Table.Head>
                    <Table.Head class="w-[120px]">Source</Table.Head>
                </Table.Row>
            </Table.Header>
            <Table.Body>
                {#if loading && runs.length === 0}
                    <Table.Row>
                        <Table.Cell
                            colspan={7}
                            class="h-32 text-center text-muted-foreground"
                        >
                            Loading runs...
                        </Table.Cell>
                    </Table.Row>
                {:else if runs.length === 0}
                    <Table.Row>
                        <Table.Cell
                            colspan={7}
                            class="h-48 text-center text-muted-foreground"
                        >
                            <div
                                class="flex flex-col items-center justify-center gap-3"
                            >
                                <div class="bg-muted p-3 rounded-full">
                                    <LayoutDashboard
                                        class="size-6 text-muted-foreground"
                                    />
                                </div>
                                <div class="space-y-1">
                                    <p class="font-medium text-foreground">
                                        No runs found
                                    </p>
                                    <p class="text-sm">
                                        There are no records matching your
                                        criteria.
                                    </p>
                                </div>
                                <Button
                                    variant="outline"
                                    class="mt-2"
                                    onclick={() => goto("/dataflows")}
                                >
                                    Go to Dataflows
                                </Button>
                            </div>
                        </Table.Cell>
                    </Table.Row>
                {:else}
                    {#each runs as run}
                        <!-- Row with detail mapping -->
                        <Table.Row
                            class="cursor-pointer hover:bg-muted/50 transition-colors group {selectedRunIds.includes(
                                run.id,
                            )
                                ? 'bg-muted/50'
                                : ''}"
                            onclick={() => rowClick(run.id)}
                        >
                            <Table.Cell
                                onclick={(e) => e.stopPropagation()}
                                class="text-center"
                            >
                                <Checkbox
                                    checked={selectedRunIds.includes(run.id)}
                                    onCheckedChange={(v: any) =>
                                        toggleSelection(run.id, v === true)}
                                    class="translate-y-[2px]"
                                />
                            </Table.Cell>
                            <Table.Cell>
                                <div class="flex flex-col">
                                    <span
                                        class="font-semibold group-hover:underline decoration-muted-foreground underline-offset-4"
                                        >{run.name}</span
                                    >
                                    <span
                                        class="font-mono text-[10px] text-muted-foreground truncate"
                                        title={run.id}
                                        >{run.id.substring(0, 12)}...</span
                                    >
                                </div>
                                {#if run.outcome_summary}
                                    <div
                                        class="text-xs text-muted-foreground mt-1 max-w-[200px] truncate"
                                        title={summarizeOutcomeSummary(run.outcome_summary)}
                                    >
                                        {summarizeOutcomeSummary(run.outcome_summary)}
                                    </div>
                                {/if}
                            </Table.Cell>
                            <Table.Cell>
                                <RunStatusBadge
                                    status={run.status}
                                    stopRequestedAt={run.stop_requested_at}
                                />
                            </Table.Cell>
                            <Table.Cell
                                class="font-mono text-xs text-muted-foreground"
                                >{formatTime(run.started_at)}</Table.Cell
                            >
                            <Table.Cell
                                class="font-mono text-xs text-muted-foreground"
                                >{formatTime(run.finished_at)}</Table.Cell
                            >
                            <Table.Cell class="text-center font-mono text-sm"
                                >{run.node_count ?? "-"}</Table.Cell
                            >
                            <Table.Cell>
                                <Badge
                                    variant="secondary"
                                    class="font-mono text-[10px] truncate max-w-[100px]"
                                    >{run.source || "unknown"}</Badge
                                >
                            </Table.Cell>
                        </Table.Row>
                    {/each}
                {/if}
            </Table.Body>
        </Table.Root>
    </div>

    <!-- Footer info -->
    <div
        class="flex justify-between items-center text-xs text-muted-foreground px-1 py-1"
    >
        <div>
            Showing {totalRuns === 0 ? 0 : (currentPage - 1) * pageSize + 1} to
            {Math.min(currentPage * pageSize, totalRuns)} of
            <span class="font-medium text-foreground">{totalRuns}</span> runs
        </div>
    </div>
</div>

<AlertDialog.Root bind:open={isDeleteDialogOpen}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>Are you absolutely sure?</AlertDialog.Title>
            <AlertDialog.Description>
                This action cannot be undone. This will permanently delete
                <strong class="text-foreground">{selectedRunIds.length}</strong>
                run instance(s) and erase all associated logs, events, and artifacts from your disk.
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
            <AlertDialog.Action
                class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                onclick={confirmDeleteSelectedRuns}>Delete</AlertDialog.Action
            >
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>
