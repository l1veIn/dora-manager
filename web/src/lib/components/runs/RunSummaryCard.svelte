<script lang="ts">
    import RunStatusBadge from "./RunStatusBadge.svelte";
    import { Badge } from "$lib/components/ui/badge/index.js";

    let { run, metrics = null } = $props<{ run: any; metrics?: any }>();

    function formatTime(ts: string) {
        if (!ts) return "-";
        return new Date(ts).toLocaleString();
    }

    function calculateDuration(start: string, end: string) {
        if (!start) return "-";
        const t1 = new Date(start).getTime();
        const t2 = end ? new Date(end).getTime() : Date.now();
        const diffMs = t2 - t1;

        const secs = Math.floor(diffMs / 1000);
        if (secs < 60) return `${secs}s`;
        const mins = Math.floor(secs / 60);
        const remSecs = secs % 60;
        return `${mins}m ${remSecs}s`;
    }
</script>

<div class="flex flex-col shrink-0">
    <div
        class="px-4 border-b bg-muted/20 flex items-center justify-between shrink-0 h-11"
    >
        <span
            class="text-[11px] font-semibold tracking-wider uppercase text-muted-foreground"
            >Run Summary</span
        >
        <Badge
            variant="secondary"
            class="font-mono text-[9px] uppercase px-1.5 py-0"
        >
            {run?.source || "unknown"}
        </Badge>
    </div>

    <div class="p-4 border-b">
        {#if !run}
            <div
                class="h-20 flex items-center justify-center text-sm text-muted-foreground"
            >
                Loading...
            </div>
        {:else}
            <!-- Primary Meta -->
            <div class="flex flex-col gap-3 mb-6 overflow-hidden">
                <div class="flex items-center justify-between text-sm gap-4">
                    <span class="text-muted-foreground shrink-0">Run ID</span>
                    <span
                        class="font-mono text-[10px] bg-muted px-1.5 py-0.5 rounded text-foreground truncate"
                        title={run.id}>{run.id}</span
                    >
                </div>
                {#if run.dora_uuid}
                    <div class="flex items-center justify-between text-sm">
                        <span class="text-muted-foreground">Dora ID</span>
                        <span
                            class="font-mono text-[10px] text-muted-foreground truncate max-w-[140px]"
                            title={run.dora_uuid}>{run.dora_uuid}</span
                        >
                    </div>
                {/if}
                <div class="flex items-center justify-between text-sm">
                    <span class="text-muted-foreground">Started</span>
                    <span class="text-[12px]">{formatTime(run.started_at)}</span
                    >
                </div>
                {#if run.status === "running" || run.finished_at}
                    <div class="flex items-center justify-between text-sm">
                        <span class="text-muted-foreground">Duration</span>
                        <span class="font-mono text-[12px]"
                            >{calculateDuration(
                                run.started_at,
                                run.finished_at,
                            )}</span
                        >
                    </div>
                {/if}
            </div>

            <dl
                class="grid grid-cols-1 gap-y-4 text-sm mt-4 pt-4 border-t border-dashed"
            >
                <div class="flex items-center justify-between">
                    <dt class="text-xs text-muted-foreground font-medium">
                        Exit Code
                    </dt>
                    <dd class="font-mono text-xs">{run.exit_code ?? "-"}</dd>
                </div>

                {#if run.log_sync?.state}
                    <div class="flex items-center justify-between">
                        <dt class="text-xs text-muted-foreground font-medium">
                            Log Sync
                        </dt>
                        <dd class="font-mono text-[10px] uppercase">
                            {run.log_sync.state}
                        </dd>
                    </div>
                {/if}

                <div class="flex items-center justify-between">
                    <dt class="text-xs text-muted-foreground font-medium">
                        Active Nodes
                    </dt>
                    <dd class="font-mono text-xs text-foreground">
                        {run.node_count_observed ?? run.node_count ?? 0} / {run.node_count_expected ??
                            run.node_count ??
                            "-"}
                    </dd>
                </div>
            </dl>

            <!-- Runtime Metrics -->
            {#if metrics && (metrics.cpu != null || metrics.memory_mb != null)}
                <dl
                    class="grid grid-cols-1 gap-y-4 text-sm mt-4 pt-4 border-t border-dashed"
                >
                    <div class="flex items-center justify-between">
                        <dt class="text-xs text-muted-foreground font-medium">
                            CPU Usage
                        </dt>
                        <dd class="font-mono text-xs text-foreground">
                            {metrics.cpu != null
                                ? `${metrics.cpu.toFixed(1)}%`
                                : "-"}
                        </dd>
                    </div>

                    <div class="flex items-center justify-between">
                        <dt class="text-xs text-muted-foreground font-medium">
                            Memory
                        </dt>
                        <dd class="font-mono text-xs text-foreground">
                            {#if metrics.memory_mb != null}
                                {metrics.memory_mb >= 1024
                                    ? `${(metrics.memory_mb / 1024).toFixed(2)} GB`
                                    : `${Math.round(metrics.memory_mb)} MB`}
                            {:else}
                                -
                            {/if}
                        </dd>
                    </div>
                </dl>
            {/if}
        {/if}
    </div>

    <!-- Transpile Meta -->
    {#if run?.transpile}
        <div class="border-b">
            <div class="px-4 py-2 bg-muted/10 border-b">
                <span
                    class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider"
                    >Transpile details</span
                >
            </div>
            <div class="p-4">
                <dl class="grid grid-cols-1 gap-y-4 text-sm">
                    {#if run.transpile.working_dir}
                        <div class="flex flex-col gap-1.5">
                            <dt
                                class="text-[11px] text-muted-foreground font-medium uppercase tracking-wider"
                            >
                                Working Dir
                            </dt>
                            <dd
                                class="font-mono text-[10px] break-all text-muted-foreground bg-muted/40 p-1.5 rounded-sm border"
                            >
                                {run.transpile.working_dir}
                            </dd>
                        </div>
                    {/if}

                    {#if run.transpile.panel_node_ids && run.transpile.panel_node_ids.length > 0}
                        <div class="flex flex-col gap-1.5">
                            <dt
                                class="text-[11px] text-muted-foreground font-medium uppercase tracking-wider"
                            >
                                Panel Nodes
                            </dt>
                            <dd
                                class="font-mono text-[10px] flex flex-wrap gap-1"
                            >
                                {#each run.transpile.panel_node_ids as nodeId}
                                    <span
                                        class="bg-indigo-50 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800 px-1.5 py-0.5 rounded"
                                        >{nodeId}</span
                                    >
                                {/each}
                            </dd>
                        </div>
                    {/if}

                    {#if run.transpile.resolved_node_paths && Object.keys(run.transpile.resolved_node_paths).length > 0}
                        <div class="flex flex-col gap-1.5">
                            <dt
                                class="text-[11px] text-muted-foreground font-medium uppercase tracking-wider"
                            >
                                Resolved Flow
                            </dt>
                            <dd class="font-mono text-[10px] space-y-1.5">
                                {#each Object.entries(run.transpile.resolved_node_paths) as [node, path]}
                                    <div
                                        class="flex flex-col bg-muted/20 p-1.5 rounded-sm border"
                                    >
                                        <span
                                            class="font-bold text-foreground mb-0.5"
                                            >{node}</span
                                        >
                                        <span
                                            class="break-all text-muted-foreground opacity-80"
                                            >{path}</span
                                        >
                                    </div>
                                {/each}
                            </dd>
                        </div>
                    {/if}
                </dl>
            </div>
        </div>
    {/if}
</div>
