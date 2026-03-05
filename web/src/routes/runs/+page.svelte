<script lang="ts">
    import { onMount } from "svelte";
    import { get, getText, del } from "$lib/api";
    import * as Table from "$lib/components/ui/table/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import {
        RefreshCw,
        ChevronLeft,
        ChevronRight,
        ChevronDown,
        ChevronUp,
        Trash2,
        FileText,
        X,
    } from "lucide-svelte";

    // State
    let runs = $state<any[]>([]);
    let loading = $state(true);
    let totalRuns = $state(0);
    let currentPage = $state(1);
    const pageSize = 20;

    // Expanded row
    let expandedRunId = $state<string | null>(null);
    let expandedDetail = $state<any | null>(null);
    let detailLoading = $state(false);

    // Log viewer
    let logContent = $state<string | null>(null);
    let logNodeId = $state("");
    let logRunId = $state("");
    let logLoading = $state(false);

    async function fetchRuns() {
        loading = true;
        try {
            const params = new URLSearchParams();
            params.append("limit", pageSize.toString());
            params.append("offset", ((currentPage - 1) * pageSize).toString());
            const result = (await get(`/runs?${params.toString()}`)) as any;
            runs = result.runs || [];
            totalRuns = result.total || 0;
        } catch (e) {
            console.error("Failed to fetch runs", e);
        } finally {
            loading = false;
        }
    }

    async function toggleExpand(runId: string) {
        if (expandedRunId === runId) {
            expandedRunId = null;
            expandedDetail = null;
            return;
        }
        expandedRunId = runId;
        detailLoading = true;
        try {
            expandedDetail = await get(`/runs/${runId}`);
        } catch (e) {
            console.error("Failed to fetch run detail", e);
            expandedDetail = null;
        } finally {
            detailLoading = false;
        }
    }

    async function viewLogs(runId: string, nodeId: string) {
        logRunId = runId;
        logNodeId = nodeId;
        logLoading = true;
        logContent = null;
        try {
            const text = await getText(`/runs/${runId}/logs/${nodeId}`);
            logContent = text;
        } catch (e) {
            logContent = "(Failed to load log)";
        } finally {
            logLoading = false;
        }
    }

    function closeLogs() {
        logContent = null;
        logNodeId = "";
        logRunId = "";
    }

    async function deleteRun(runId: string) {
        if (!confirm(`Delete run ${runId}?`)) return;
        try {
            await del(`/runs/${runId}`);
            if (expandedRunId === runId) {
                expandedRunId = null;
                expandedDetail = null;
            }
            fetchRuns();
        } catch (e) {
            console.error("Failed to delete run", e);
        }
    }

    function formatTime(ts: string) {
        if (!ts) return "-";
        return new Date(ts).toLocaleString();
    }

    function formatSize(bytes: number) {
        if (bytes === 0) return "(empty)";
        if (bytes < 1024) return `${bytes} B`;
        return `${(bytes / 1024).toFixed(1)} KB`;
    }

    onMount(() => {
        fetchRuns();
    });
</script>

<div class="p-6 max-w-6xl mx-auto space-y-4 h-full flex flex-col">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-3xl font-bold tracking-tight">Runs</h1>
            <p class="text-sm text-muted-foreground">
                Dataflow execution history and node logs.
            </p>
        </div>

        <Button variant="outline" size="sm" onclick={fetchRuns}>
            <RefreshCw class="mr-2 size-4 {loading ? 'animate-spin' : ''}" /> Refresh
        </Button>
    </div>

    <div class="border rounded-md shrink-0 overflow-auto bg-card flex-1">
        <Table.Root>
            <Table.Header class="sticky top-0 bg-card z-10 shadow-sm">
                <Table.Row>
                    <Table.Head class="w-[40px]"></Table.Head>
                    <Table.Head class="w-[120px]">Name</Table.Head>
                    <Table.Head class="w-[180px]">Started</Table.Head>
                    <Table.Head class="w-[180px]">Finished</Table.Head>
                    <Table.Head class="w-[80px]">Status</Table.Head>
                    <Table.Head class="w-[80px]">Nodes</Table.Head>
                    <Table.Head class="w-[280px]">Run ID</Table.Head>
                    <Table.Head class="w-[60px]"></Table.Head>
                </Table.Row>
            </Table.Header>
            <Table.Body>
                {#if loading && runs.length === 0}
                    <Table.Row>
                        <Table.Cell colspan={8} class="h-24 text-center"
                            >Loading runs...</Table.Cell
                        >
                    </Table.Row>
                {:else if runs.length === 0}
                    <Table.Row>
                        <Table.Cell
                            colspan={8}
                            class="h-24 text-center text-muted-foreground"
                            >No runs recorded yet. Start a dataflow with <code
                                >dm start</code
                            > to see history here.</Table.Cell
                        >
                    </Table.Row>
                {:else}
                    {#each runs as run}
                        <Table.Row
                            class="cursor-pointer hover:bg-muted/50 transition-colors"
                            onclick={() => toggleExpand(run.id)}
                        >
                            <Table.Cell class="px-2">
                                {#if expandedRunId === run.id}
                                    <ChevronUp
                                        class="size-4 text-muted-foreground"
                                    />
                                {:else}
                                    <ChevronDown
                                        class="size-4 text-muted-foreground"
                                    />
                                {/if}
                            </Table.Cell>
                            <Table.Cell class="font-semibold"
                                >{run.name}</Table.Cell
                            >
                            <Table.Cell
                                class="font-mono text-xs text-muted-foreground"
                                >{formatTime(run.started_at)}</Table.Cell
                            >
                            <Table.Cell
                                class="font-mono text-xs text-muted-foreground"
                                >{formatTime(run.finished_at)}</Table.Cell
                            >
                            <Table.Cell>
                                {#if run.exit_code === null || run.exit_code === undefined}
                                    <Badge
                                        variant="outline"
                                        class="text-[10px] font-mono"
                                        >running</Badge
                                    >
                                {:else if run.exit_code === 0}
                                    <Badge
                                        variant="default"
                                        class="text-[10px] font-mono bg-green-600"
                                        >success</Badge
                                    >
                                {:else}
                                    <Badge
                                        variant="destructive"
                                        class="text-[10px] font-mono"
                                        >exit {run.exit_code}</Badge
                                    >
                                {/if}
                            </Table.Cell>
                            <Table.Cell class="text-center"
                                >{run.node_count}</Table.Cell
                            >
                            <Table.Cell
                                class="font-mono text-[11px] text-muted-foreground truncate max-w-[280px]"
                                title={run.id}>{run.id}</Table.Cell
                            >
                            <Table.Cell>
                                <div
                                    class="flex items-center justify-center gap-1"
                                >
                                    <a
                                        href="/panel?run={run.id}"
                                        class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground size-7"
                                        title="View panel data"
                                        onclick={(e) => e.stopPropagation()}
                                    >
                                        📊
                                    </a>
                                    <Button
                                        variant="ghost"
                                        size="icon"
                                        class="size-7"
                                        onclick={(e) => {
                                            e.stopPropagation();
                                            deleteRun(run.id);
                                        }}
                                    >
                                        <Trash2
                                            class="size-3.5 text-muted-foreground hover:text-destructive"
                                        />
                                    </Button>
                                </div>
                            </Table.Cell>
                        </Table.Row>
                        {#if expandedRunId === run.id}
                            <Table.Row>
                                <Table.Cell colspan={8} class="p-0">
                                    <div class="bg-muted/30 border-t px-6 py-3">
                                        {#if detailLoading}
                                            <p
                                                class="text-sm text-muted-foreground"
                                            >
                                                Loading...
                                            </p>
                                        {:else if expandedDetail?.nodes?.length > 0}
                                            <p
                                                class="text-xs font-medium text-muted-foreground mb-2"
                                            >
                                                Node Logs
                                            </p>
                                            <div
                                                class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2"
                                            >
                                                {#each expandedDetail.nodes as node}
                                                    <button
                                                        class="flex items-center gap-2 px-3 py-2 rounded-md border bg-card text-sm hover:bg-accent transition-colors text-left"
                                                        onclick={() =>
                                                            viewLogs(
                                                                run.id,
                                                                node.id,
                                                            )}
                                                    >
                                                        <FileText
                                                            class="size-3.5 text-muted-foreground shrink-0"
                                                        />
                                                        <span
                                                            class="font-medium truncate"
                                                            >{node.id}</span
                                                        >
                                                        <span
                                                            class="text-xs text-muted-foreground ml-auto"
                                                            >{formatSize(
                                                                node.log_size,
                                                            )}</span
                                                        >
                                                    </button>
                                                {/each}
                                            </div>
                                        {:else}
                                            <p
                                                class="text-sm text-muted-foreground"
                                            >
                                                No log files found for this run.
                                            </p>
                                        {/if}
                                    </div>
                                </Table.Cell>
                            </Table.Row>
                        {/if}
                    {/each}
                {/if}
            </Table.Body>
        </Table.Root>
    </div>

    <!-- Pagination -->
    <div class="flex items-center justify-between px-2 pt-2">
        <div class="text-sm text-muted-foreground">
            Showing {totalRuns === 0 ? 0 : (currentPage - 1) * pageSize + 1} to
            {Math.min(currentPage * pageSize, totalRuns)} of
            <span class="font-medium text-foreground">{totalRuns}</span> runs
        </div>
        <div class="flex items-center gap-2">
            <Button
                variant="outline"
                size="sm"
                disabled={currentPage === 1 || loading}
                onclick={() => {
                    currentPage--;
                    fetchRuns();
                }}
            >
                <ChevronLeft class="size-4 mr-1" /> Previous
            </Button>
            <Button
                variant="outline"
                size="sm"
                disabled={currentPage * pageSize >= totalRuns || loading}
                onclick={() => {
                    currentPage++;
                    fetchRuns();
                }}
            >
                Next <ChevronRight class="size-4 ml-1" />
            </Button>
        </div>
    </div>
</div>

<!-- Log Viewer Modal -->
{#if logContent !== null}
    <div
        class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
        role="dialog"
    >
        <div
            class="bg-card border rounded-lg shadow-xl w-[90vw] max-w-4xl max-h-[80vh] flex flex-col"
        >
            <div class="flex items-center justify-between px-4 py-3 border-b">
                <div class="text-sm font-medium">
                    <span class="text-muted-foreground">Log:</span>
                    {logNodeId}
                    <span class="text-muted-foreground ml-2 text-xs font-mono"
                        >{logRunId}</span
                    >
                </div>
                <Button
                    variant="ghost"
                    size="icon"
                    class="size-7"
                    onclick={closeLogs}
                >
                    <X class="size-4" />
                </Button>
            </div>
            <div class="flex-1 overflow-auto p-4">
                {#if logLoading}
                    <p class="text-sm text-muted-foreground">Loading...</p>
                {:else}
                    <pre
                        class="text-xs font-mono whitespace-pre-wrap break-all text-muted-foreground">{logContent}</pre>
                {/if}
            </div>
        </div>
    </div>
{/if}
