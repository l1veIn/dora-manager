<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { ChevronDown, LayoutGrid } from "lucide-svelte";
    import ControlButton from "./controls/ControlButton.svelte";
    import ControlCheckbox from "./controls/ControlCheckbox.svelte";
    import ControlInput from "./controls/ControlInput.svelte";
    import ControlPath from "./controls/ControlPath.svelte";
    import ControlRadio from "./controls/ControlRadio.svelte";
    import ControlSelect from "./controls/ControlSelect.svelte";
    import ControlSlider from "./controls/ControlSlider.svelte";
    import ControlSwitch from "./controls/ControlSwitch.svelte";
    import ControlTextarea from "./controls/ControlTextarea.svelte";
    import type { PanelRendererProps } from "../types";
    import { createSnapshotViewState, summarizeSelection } from "../message/message-state.svelte.js";

    let {
        item,
        context,
        onConfigChange,
    }: PanelRendererProps = $props();

    function ensureConfig() {
        if (!item.config) item.config = {};
        if (!Array.isArray(item.config.nodes) || item.config.nodes.length === 0) {
            item.config.nodes = ["*"];
        }
        if (!Array.isArray(item.config.tags) || item.config.tags.length === 0) {
            item.config.tags = ["widgets"];
        }
        if (typeof item.config.gridCols !== "number" || ![1, 2, 3].includes(item.config.gridCols)) {
            item.config.gridCols = 2;
        }
    }

    ensureConfig();
    let selectedNodes = $derived(Array.isArray(item.config.nodes) ? item.config.nodes : ["*"]);
    let selectedTags = $derived(Array.isArray(item.config.tags) ? item.config.tags : ["widgets"]);
    let gridCols = $derived(typeof item.config.gridCols === "number" ? item.config.gridCols : 2);
    let availableSources = $derived(
        context.snapshots
            .filter((snapshot: any) => snapshot.tag === "widgets")
            .map((snapshot: any) => snapshot.node_id)
            .filter((value: string, index: number, items: string[]) => value && items.indexOf(value) === index),
    );
    const snapshotView = createSnapshotViewState(
        () => context.snapshots,
        () => ({ nodes: selectedNodes, tags: selectedTags }),
    );
    let widgetSnapshots = $derived(snapshotView.snapshots);
    let gridClass = $derived.by(() => {
        if (gridCols === 1) return "grid-cols-1";
        if (gridCols === 3) return "grid-cols-1 lg:grid-cols-2 2xl:grid-cols-3";
        return "grid-cols-1 xl:grid-cols-2";
    });

    let draftValues = $state<Record<string, any>>({});
    let sendingId = $state<string | null>(null);

    function applyConfig() {
        onConfigChange?.();
    }

    function widgetKey(nodeId: string, outputId: string) {
        return `${nodeId}:${outputId}`;
    }

    function widgetEntries(binding: any): Array<[string, any]> {
        return Object.entries(binding.payload?.widgets ?? {}) as Array<[string, any]>;
    }

    function shouldShowWidgetLabel(binding: any) {
        return widgetEntries(binding).length > 1;
    }

    function initialValue(binding: any, outputId: string, widget: any) {
        const key = widgetKey(binding.node_id, outputId);
        if (draftValues[key] !== undefined) return draftValues[key];
        if (context.inputValues[key] !== undefined) return context.inputValues[key];
        if (widget?.default !== undefined) return widget.default;
        if (widget?.type === "checkbox") return [];
        if (widget?.type === "switch") return false;
        if (widget?.type === "slider") return widget?.min ?? 0;
        return "";
    }

    async function handleEmit(nodeId: string, outputId: string, value: any) {
        const key = widgetKey(nodeId, outputId);
        draftValues[key] = value;
        sendingId = key;
        try {
            await context.emitMessage({
                from: "web",
                tag: "input",
                payload: {
                    to: nodeId,
                    output_id: outputId,
                    value,
                },
            });
        } finally {
            if (sendingId === key) sendingId = null;
        }
    }

    async function emitFile(nodeId: string, outputId: string, fileList: FileList | null) {
        const file = fileList?.[0];
        if (!file) return;
        const buf = await file.arrayBuffer();
        const bytes = new Uint8Array(buf);
        let binary = "";
        for (const byte of bytes) binary += String.fromCharCode(byte);
        await handleEmit(nodeId, outputId, btoa(binary));
    }

    function setAllNodes() {
        item.config.nodes = ["*"];
        applyConfig();
    }

    function toggleNode(value: string) {
        const current = selectedNodes.includes("*") ? [] : [...selectedNodes];
        const next = current.includes(value)
            ? current.filter((item) => item !== value)
            : [...current, value];
        item.config.nodes = next.length > 0 ? next : ["*"];
        applyConfig();
    }

    function setGridCols(value: number) {
        item.config.gridCols = value;
        applyConfig();
    }

    function handleMenuSelect(event: Event, action: () => void) {
        event.preventDefault();
        action();
    }
</script>

<div class="flex h-full w-full flex-col overflow-hidden bg-background">
    <div class="px-3 h-8 border-b bg-muted/20 flex items-center justify-between shrink-0">
        <div class="flex-1"></div>
        <div class="flex items-center gap-1.5">
            <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                        <Button {...props} variant="ghost" size="sm" class="h-7 w-auto max-w-[156px] justify-between gap-2 rounded-full border-0 bg-muted/20 px-2.5 text-[11px] font-mono text-foreground/90 shadow-none hover:bg-muted/35">
                            <span class="min-w-0 truncate">{summarizeSelection(selectedNodes, "All Sources")}</span>
                            <ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
                        </Button>
                    {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end" class="w-56">
                    <DropdownMenu.Label>Filter Sources</DropdownMenu.Label>
                    <DropdownMenu.Separator />
                    <DropdownMenu.CheckboxItem checked={selectedNodes.includes("*")} onclick={(event) => handleMenuSelect(event, setAllNodes)}>
                        All Sources
                    </DropdownMenu.CheckboxItem>
                    <DropdownMenu.Separator />
                    {#each availableSources as nodeId}
                        <DropdownMenu.CheckboxItem checked={!selectedNodes.includes("*") && selectedNodes.includes(nodeId)} onclick={(event) => handleMenuSelect(event, () => toggleNode(nodeId))}>
                            {nodeId}
                        </DropdownMenu.CheckboxItem>
                    {/each}
                </DropdownMenu.Content>
            </DropdownMenu.Root>

            <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                        <Button {...props} variant="ghost" size="sm" class="h-7 w-auto max-w-[132px] justify-between gap-2 rounded-full border-0 bg-muted/20 px-2.5 text-[11px] font-mono text-foreground/90 shadow-none hover:bg-muted/35">
                            <span class="flex min-w-0 items-center gap-1 overflow-hidden">
                                <LayoutGrid class="size-3 shrink-0 text-muted-foreground/70" />
                                <span class="truncate">{gridCols} Cols</span>
                            </span>
                            <ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
                        </Button>
                    {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end" class="w-44">
                    <DropdownMenu.Label>Grid Layout</DropdownMenu.Label>
                    <DropdownMenu.Separator />
                    <DropdownMenu.CheckboxItem checked={gridCols === 1} onclick={(event) => handleMenuSelect(event, () => setGridCols(1))}>
                        1 Column
                    </DropdownMenu.CheckboxItem>
                    <DropdownMenu.CheckboxItem checked={gridCols === 2} onclick={(event) => handleMenuSelect(event, () => setGridCols(2))}>
                        2 Columns
                    </DropdownMenu.CheckboxItem>
                    <DropdownMenu.CheckboxItem checked={gridCols === 3} onclick={(event) => handleMenuSelect(event, () => setGridCols(3))}>
                        3 Columns
                    </DropdownMenu.CheckboxItem>
                </DropdownMenu.Content>
            </DropdownMenu.Root>
        </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4 space-y-4 bg-muted/10">
        {#if widgetSnapshots.length === 0}
            <div class="flex min-h-[220px] items-center justify-center rounded-xl border border-dashed bg-background/60 text-sm text-muted-foreground">
                No input controls available.
            </div>
        {/if}

        <div class={`grid gap-3 ${gridClass}`}>
            {#each widgetSnapshots as binding}
                <div class="rounded-lg border bg-background/95 overflow-hidden shadow-[0_1px_2px_rgba(0,0,0,0.04)]">
                    <div class="px-3 pt-2.5 pb-1.5">
                        <div class="truncate font-medium text-[13px] leading-5">{binding.payload?.label || binding.node_id}</div>
                        <div class="truncate text-[10px] text-muted-foreground/80 font-mono leading-4">{binding.node_id}</div>
                    </div>
                    <div class="px-3 pb-3 space-y-2.5">
                        {#each widgetEntries(binding) as [outputId, widget]}
                            <div class="space-y-1">
                                {#if shouldShowWidgetLabel(binding)}
                                    <div class="text-[9px] font-medium text-muted-foreground/75 uppercase tracking-[0.18em]">
                                        {widget.label ?? outputId}
                                    </div>
                                {/if}
                                {#if widget.type === "textarea"}
                                    <ControlTextarea {outputId} xw={widget} label={widget.label ?? outputId} value={initialValue(binding, outputId, widget)} disabled={!context.isRunActive} sending={sendingId === widgetKey(binding.node_id, outputId)} onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v} onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))} />
                                {:else if widget.type === "input"}
                                    <ControlInput {outputId} xw={widget} label={widget.label ?? outputId} value={initialValue(binding, outputId, widget)} disabled={!context.isRunActive} sending={sendingId === widgetKey(binding.node_id, outputId)} onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v} onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))} />
                                {:else if widget.type === "button"}
                                    <ControlButton {outputId} xw={widget} label={widget.label ?? outputId} disabled={!context.isRunActive} sending={sendingId === widgetKey(binding.node_id, outputId)} onSend={(v) => handleEmit(binding.node_id, outputId, v)} />
                                {:else if widget.type === "select"}
                                    <ControlSelect {outputId} options={widget.options ?? []} value={draftValues[widgetKey(binding.node_id, outputId)]} defaultValue={initialValue(binding, outputId, widget)} disabled={!context.isRunActive} onValueChange={(v) => handleEmit(binding.node_id, outputId, v)} />
                                {:else if widget.type === "slider"}
                                    <ControlSlider {outputId} xw={widget} value={draftValues[widgetKey(binding.node_id, outputId)]} defaultValue={initialValue(binding, outputId, widget)} disabled={!context.isRunActive} onValueChange={(v) => handleEmit(binding.node_id, outputId, v)} />
                                {:else if widget.type === "switch"}
                                    <ControlSwitch {outputId} xw={widget} value={draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget)} disabled={!context.isRunActive} onValueChange={(v) => handleEmit(binding.node_id, outputId, v)} />
                                {:else if widget.type === "radio"}
                                    <ControlRadio {outputId} options={widget.options ?? []} label={widget.label ?? outputId} value={draftValues[widgetKey(binding.node_id, outputId)]} defaultValue={initialValue(binding, outputId, widget)} disabled={!context.isRunActive} sending={sendingId === widgetKey(binding.node_id, outputId)} onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v} onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))} />
                                {:else if widget.type === "checkbox"}
                                    <ControlCheckbox {outputId} options={widget.options ?? []} label={widget.label ?? outputId} value={draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget)} disabled={!context.isRunActive} sending={sendingId === widgetKey(binding.node_id, outputId)} onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v} onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))} />
                                {:else if widget.type === "path" || widget.type === "file_picker" || widget.type === "directory"}
                                    <ControlPath {outputId} xw={widget} mode={widget.type === "directory" ? "directory" : "file"} value={draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget)} disabled={!context.isRunActive} sending={sendingId === widgetKey(binding.node_id, outputId)} onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v} onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))} />
                                {:else if widget.type === "file"}
                                    <input type="file" disabled={!context.isRunActive} class="text-sm cursor-pointer file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-primary file:text-primary-foreground hover:file:bg-primary/90 disabled:cursor-not-allowed" onchange={(e) => emitFile(binding.node_id, outputId, (e.currentTarget as HTMLInputElement).files)} />
                                {:else}
                                    <div class="rounded-md border border-dashed p-3 text-sm text-muted-foreground bg-muted/50">Unsupported widget type: {widget.type ?? "unknown"}</div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                </div>
            {/each}
        </div>
    </div>
</div>
