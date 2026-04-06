<script lang="ts">
    import { BarChart, LineChart } from "layerchart";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { ChevronDown } from "lucide-svelte";
    import type { PanelRendererProps } from "../types";
    import { createSnapshotViewState, summarizeSelection } from "../message/message-state.svelte.js";

    type ChartSeries = {
        name?: string;
        data?: Array<number | null>;
        color?: string;
    };

    type ChartPayload = {
        title?: string;
        description?: string;
        type?: "line" | "bar";
        labels?: string[];
        series?: ChartSeries[];
    };

    let { item, context, onConfigChange }: PanelRendererProps = $props();

    function ensureConfig() {
        if (!item.config) item.config = {};
        if (!Array.isArray(item.config.nodes) || item.config.nodes.length === 0) {
            item.config.nodes = ["*"];
        }
        if (!Array.isArray(item.config.tags) || item.config.tags.length === 0) {
            item.config.tags = ["chart"];
        }
    }

    ensureConfig();

    let selectedNodes = $derived(Array.isArray(item.config.nodes) ? item.config.nodes : ["*"]);
    let selectedTags = $derived(Array.isArray(item.config.tags) ? item.config.tags : ["chart"]);
    let availableNodes = $derived(
        context.snapshots
            .map((snapshot: any) => snapshot.node_id)
            .filter((value: string, index: number, items: string[]) => value && items.indexOf(value) === index),
    );

    const snapshotView = createSnapshotViewState(
        () => context.snapshots,
        () => ({ nodes: selectedNodes, tags: selectedTags }),
    );
    let chartSnapshots = $derived(snapshotView.snapshots);

    function applyFilters() {
        onConfigChange?.();
    }

    function setAllNodes() {
        item.config.nodes = ["*"];
        applyFilters();
    }

    function toggleNode(value: string) {
        const current = selectedNodes.includes("*") ? [] : [...selectedNodes];
        const next = current.includes(value)
            ? current.filter((item) => item !== value)
            : [...current, value];
        item.config.nodes = next.length > 0 ? next : ["*"];
        applyFilters();
    }

    function handleMenuSelect(event: Event, action: () => void) {
        event.preventDefault();
        action();
    }

    function normalizePayload(snapshot: any): ChartPayload {
        return snapshot.payload ?? {};
    }

    function dataset(payload: ChartPayload) {
        const labels = Array.isArray(payload.labels) ? payload.labels : [];
        const series = Array.isArray(payload.series) ? payload.series : [];
        return labels.map((label, index) => {
            const row: Record<string, string | number | null> = { label };
            for (const [seriesIndex, entry] of series.entries()) {
                row[`series_${seriesIndex}`] = entry.data?.[index] ?? null;
            }
            return row;
        });
    }

    function seriesDefs(payload: ChartPayload) {
        const series = Array.isArray(payload.series) ? payload.series : [];
        return series.map((entry, index) => ({
            key: `series_${index}`,
            label: entry.name ?? `Series ${index + 1}`,
            value: `series_${index}`,
            color: entry.color ?? `var(--color-chart-${(index % 5) + 1})`,
        }));
    }

    function yKeys(payload: ChartPayload) {
        return seriesDefs(payload).map((entry) => entry.value);
    }
</script>

<div class="h-full w-full overflow-y-auto p-4 space-y-4 bg-muted/10">
    <div class="flex items-center justify-between gap-3">
        <div class="flex-1"></div>
        <div class="flex items-center gap-1.5">
            <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                        <Button {...props} variant="ghost" size="sm" class="h-7 w-auto max-w-[156px] justify-between gap-2 rounded-full border-0 bg-muted/20 px-2.5 text-[11px] font-mono text-foreground/90 shadow-none hover:bg-muted/35">
                            <span class="min-w-0 truncate">{summarizeSelection(selectedNodes, "All Nodes")}</span>
                            <ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
                        </Button>
                    {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end" class="w-56">
                    <DropdownMenu.Label>Filter Nodes</DropdownMenu.Label>
                    <DropdownMenu.Separator />
                    <DropdownMenu.CheckboxItem checked={selectedNodes.includes("*")} onclick={(event) => handleMenuSelect(event, setAllNodes)}>
                        All Nodes
                    </DropdownMenu.CheckboxItem>
                    <DropdownMenu.Separator />
                    {#each availableNodes as nodeId}
                        <DropdownMenu.CheckboxItem checked={!selectedNodes.includes("*") && selectedNodes.includes(nodeId)} onclick={(event) => handleMenuSelect(event, () => toggleNode(nodeId))}>
                            {nodeId}
                        </DropdownMenu.CheckboxItem>
                    {/each}
                </DropdownMenu.Content>
            </DropdownMenu.Root>
        </div>
    </div>

    {#if chartSnapshots.length === 0}
        <div class="flex min-h-[220px] items-center justify-center rounded-xl border border-dashed bg-background/60 text-sm text-muted-foreground">
            No chart snapshots available.
        </div>
    {/if}

    <div class="grid gap-4">
        {#each chartSnapshots as snapshot (snapshot.node_id + ":" + snapshot.seq)}
            {@const payload = normalizePayload(snapshot)}
            {@const data = dataset(payload)}
            {@const series = seriesDefs(payload)}
            <section class="rounded-xl border bg-background shadow-sm overflow-hidden">
                <header class="border-b bg-muted/20 px-4 py-3">
                    <div class="flex items-center justify-between gap-3">
                        <div>
                            <div class="text-sm font-semibold">{payload.title ?? snapshot.node_id}</div>
                            <div class="text-[11px] font-mono text-muted-foreground">{snapshot.node_id}</div>
                        </div>
                        <Badge variant="outline" class="text-[10px] uppercase tracking-[0.18em]">
                            {payload.type ?? "line"}
                        </Badge>
                    </div>
                    {#if payload.description}
                        <p class="mt-1 text-xs text-muted-foreground">{payload.description}</p>
                    {/if}
                </header>

                <div class="p-4">
                    {#if data.length === 0 || series.length === 0}
                        <div class="flex min-h-[180px] items-center justify-center rounded-lg border border-dashed bg-muted/20 text-sm text-muted-foreground">
                            Invalid chart payload. Expected `labels` and `series`.
                        </div>
                    {:else if payload.type === "bar"}
                        <div class="h-[260px] w-full overflow-hidden rounded-lg border bg-card p-2">
                            <BarChart
                                data={data}
                                x="label"
                                y={yKeys(payload)}
                                series={series}
                                seriesLayout="group"
                                grid={true}
                                legend={true}
                                axis={true}
                            />
                        </div>
                    {:else}
                        <div class="h-[260px] w-full overflow-hidden rounded-lg border bg-card p-2">
                            <LineChart
                                data={data}
                                x="label"
                                y={yKeys(payload)}
                                series={series}
                                grid={true}
                                legend={true}
                                axis={true}
                            />
                        </div>
                    {/if}
                </div>
            </section>
        {/each}
    </div>
</div>
