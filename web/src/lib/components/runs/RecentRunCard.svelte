<script lang="ts">
    import * as Card from "$lib/components/ui/card/index.js";
    import RunStatusBadge from "./RunStatusBadge.svelte";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Clock, Activity } from "lucide-svelte";
    import { goto } from "$app/navigation";
    import { summarizeOutcomeSummary } from "$lib/runs/outcomeSummary";

    let { run } = $props<{ run: any }>();

    function formatTimeAgo(ts: string) {
        if (!ts) return "-";
        const date = new Date(ts);
        const diffMs = Date.now() - date.getTime();
        const diffSecs = Math.floor(diffMs / 1000);

        if (diffSecs < 60) return `${diffSecs}s ago`;
        const diffMins = Math.floor(diffSecs / 60);
        if (diffMins < 60) return `${diffMins}m ago`;
        const diffHours = Math.floor(diffMins / 60);
        if (diffHours < 24) return `${diffHours}h ago`;
        const diffDays = Math.floor(diffHours / 24);
        return `${diffDays}d ago`;
    }
</script>

<Card.Root
    class="hover:bg-muted/50 transition-colors cursor-pointer border-l-4 {(run.status ===
    'running' && !run.stop_requested_at)
        ? 'border-l-blue-500'
        : run.status === 'running' && run.stop_requested_at
          ? 'border-l-amber-500'
        : run.status === 'succeeded'
          ? 'border-l-emerald-500'
          : run.status === 'failed'
            ? 'border-l-red-500'
            : 'border-l-muted-foreground'} relative h-full flex flex-col"
    onclick={() => goto(`/runs/${run.id}`)}
>
    <Card.Header class="p-4 pb-2 shrink-0">
        <div class="flex items-start justify-between">
            <div class="space-y-1 min-w-0 pr-2 flex-1">
                <Card.Title
                    class="text-[14px] leading-tight font-semibold flex items-center gap-2 truncate"
                >
                    <span class="truncate">{run.name}</span>
                    {#if run.status === "running" && !run.stop_requested_at}
                        <Activity
                            class="size-3.5 text-blue-500 animate-pulse shrink-0"
                        />
                    {/if}
                </Card.Title>
                <div
                    class="font-mono text-[10px] text-muted-foreground truncate"
                    title={run.id}
                >
                    {run.id.substring(0, 8)}...
                </div>
            </div>
            <div class="shrink-0">
                <RunStatusBadge
                    status={run.status}
                    stopRequestedAt={run.stop_requested_at}
                />
            </div>
        </div>
    </Card.Header>
    <Card.Content class="p-4 pt-1 flex flex-col gap-3 flex-1 justify-between">
        {#if run.outcome_summary}
            <p
                class="text-xs text-muted-foreground line-clamp-2"
                title={summarizeOutcomeSummary(run.outcome_summary)}
            >
                {summarizeOutcomeSummary(run.outcome_summary)}
            </p>
        {:else}
            <div class="h-4"></div>
        {/if}

        <div
            class="flex items-center gap-x-4 gap-y-2 text-xs text-muted-foreground flex-wrap mt-auto"
        >
            <div class="flex items-center gap-1.5" title="Started at">
                <Clock class="size-3.5" />
                <span>{formatTimeAgo(run.started_at)}</span>
            </div>
            {#if run.node_count !== undefined}
                <div class="flex items-center gap-1.5">
                    <span class="font-mono bg-muted px-1 rounded"
                        >{run.node_count}</span
                    > nodes
                </div>
            {/if}

            {#if run.metrics && (run.metrics.cpu != null || run.metrics.memory_mb != null)}
                <div
                    class="flex items-center gap-3 border-l border-muted-foreground/20 pl-4 ml-auto md:ml-0"
                >
                    {#if run.metrics.cpu != null}
                        <div
                            class="flex items-center gap-1.5"
                            title="CPU Usage"
                        >
                            <span
                                class="text-[10px] font-semibold uppercase text-orange-600/70 dark:text-orange-400/70 tracking-wider"
                                >CPU</span
                            >
                            <span
                                class="font-mono text-orange-600 dark:text-orange-400"
                                >{run.metrics.cpu.toFixed(1)}%</span
                            >
                        </div>
                    {/if}
                    {#if run.metrics.memory_mb != null}
                        <div
                            class="flex items-center gap-1.5"
                            title="Memory Usage"
                        >
                            <span
                                class="text-[10px] font-semibold uppercase text-blue-600/70 dark:text-blue-400/70 tracking-wider"
                                >RAM</span
                            >
                            <span
                                class="font-mono text-blue-600 dark:text-blue-400"
                                >{run.metrics.memory_mb >= 1024
                                    ? `${(run.metrics.memory_mb / 1024).toFixed(1)}GB`
                                    : `${Math.round(run.metrics.memory_mb)}MB`}</span
                            >
                        </div>
                    {/if}
                </div>
            {/if}
        </div>
    </Card.Content>
</Card.Root>
