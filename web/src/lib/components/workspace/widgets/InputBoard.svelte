<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";

    import ControlButton from "./controls/ControlButton.svelte";
    import ControlCheckbox from "./controls/ControlCheckbox.svelte";
    import ControlInput from "./controls/ControlInput.svelte";
    import ControlPath from "./controls/ControlPath.svelte";
    import ControlRadio from "./controls/ControlRadio.svelte";
    import ControlSelect from "./controls/ControlSelect.svelte";
    import ControlSlider from "./controls/ControlSlider.svelte";
    import ControlSwitch from "./controls/ControlSwitch.svelte";
    import ControlTextarea from "./controls/ControlTextarea.svelte";

    let { node, api, runId, inputs = [], onEmit } = $props<{ 
        node: any; 
        api: any; 
        runId: string; 
        inputs: any[];
        onEmit: any;
    }>();

    let filteredInputs = $derived(inputs.filter((i: any) => 
        !node.config.subscribedInputs || 
        node.config.subscribedInputs.length === 0 || 
        node.config.subscribedInputs.includes(i.node_id)
    ));

    let draftValues = $state<Record<string, any>>({});
    let sendingId = $state<string | null>(null);

    function widgetKey(nodeId: string, outputId: string) {
        return `${nodeId}:${outputId}`;
    }

    function optionValue(option: any) {
        return typeof option === "object" ? option.value : option;
    }

    function optionLabel(option: any) {
        return typeof option === "object" ? option.label ?? option.value : option;
    }

    function widgetEntries(binding: any): Array<[string, any]> {
        return Object.entries(binding.widgets ?? {}) as Array<[string, any]>;
    }

    function initialValue(binding: any, outputId: string, widget: any) {
        const key = widgetKey(binding.node_id, outputId);
        if (draftValues[key] !== undefined) return draftValues[key];
        if (binding.current_values?.[outputId] !== undefined) return binding.current_values[outputId];
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
            await onEmit(nodeId, outputId, value);
        } finally {
            if (sendingId === key) {
                sendingId = null;
            }
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

    function toggleCheckbox(binding: any, outputId: string, option: any, checked: boolean) {
        const current = [...(initialValue(binding, outputId, binding.widgets[outputId]) ?? [])];
        const value = optionValue(option);
        const next = checked ? [...current.filter((item) => item !== value), value] : current.filter((item) => item !== value);
        handleEmit(binding.node_id, outputId, next);
    }

</script>

<div class="h-full w-full overflow-y-auto p-4 space-y-4 bg-muted/10">
    {#if filteredInputs.length === 0}
        <div class="flex flex-col items-center justify-center h-full text-sm text-muted-foreground">
            No input controls available.
        </div>
    {/if}

    <div class="grid xl:grid-cols-2 gap-4">
        {#each filteredInputs as binding}
            <div class="rounded-lg border bg-background overflow-hidden shadow-sm">
                <div class="px-3 py-2 border-b bg-muted/20">
                    <div class="font-medium text-sm">{binding.label || binding.node_id}</div>
                    <div class="text-[11px] text-muted-foreground font-mono">{binding.node_id}</div>
                </div>
                <div class="p-3 space-y-4">
                    {#each widgetEntries(binding) as [outputId, widget]}
                        <div class="space-y-2">
                            <div class="text-xs font-medium text-muted-foreground uppercase tracking-wider">{widget.label ?? outputId}</div>
                            {#if widget.type === "textarea"}
                                <ControlTextarea
                                    {outputId} xw={widget} label={widget.label ?? outputId}
                                    value={initialValue(binding, outputId, widget)} disabled={false} sending={sendingId === widgetKey(binding.node_id, outputId)}
                                    onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v}
                                    onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))}
                                />
                            {:else if widget.type === "input"}
                                <ControlInput
                                    {outputId} xw={widget} label={widget.label ?? outputId}
                                    value={initialValue(binding, outputId, widget)} disabled={false} sending={sendingId === widgetKey(binding.node_id, outputId)}
                                    onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v}
                                    onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))}
                                />
                            {:else if widget.type === "button"}
                                <ControlButton
                                    {outputId} xw={widget} label={widget.label ?? outputId}
                                    disabled={false} sending={sendingId === widgetKey(binding.node_id, outputId)}
                                    onSend={(v) => handleEmit(binding.node_id, outputId, v)}
                                />
                            {:else if widget.type === "select"}
                                <ControlSelect
                                    {outputId} options={widget.options ?? []}
                                    value={draftValues[widgetKey(binding.node_id, outputId)]}
                                    defaultValue={initialValue(binding, outputId, widget)}
                                    disabled={false}
                                    onValueChange={(v) => handleEmit(binding.node_id, outputId, v)}
                                />
                            {:else if widget.type === "slider"}
                                <ControlSlider
                                    {outputId} xw={widget}
                                    value={draftValues[widgetKey(binding.node_id, outputId)]}
                                    defaultValue={initialValue(binding, outputId, widget)}
                                    disabled={false}
                                    onValueChange={(v) => handleEmit(binding.node_id, outputId, v)}
                                />
                            {:else if widget.type === "switch"}
                                <ControlSwitch
                                    {outputId} xw={widget}
                                    value={draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget)}
                                    disabled={false}
                                    onValueChange={(v) => handleEmit(binding.node_id, outputId, v)}
                                />
                            {:else if widget.type === "radio"}
                                <ControlRadio
                                    {outputId} options={widget.options ?? []} label={widget.label ?? outputId}
                                    value={draftValues[widgetKey(binding.node_id, outputId)]}
                                    defaultValue={initialValue(binding, outputId, widget)}
                                    disabled={false} sending={sendingId === widgetKey(binding.node_id, outputId)}
                                    onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v}
                                    onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))}
                                />
                            {:else if widget.type === "checkbox"}
                                <ControlCheckbox
                                    {outputId} options={widget.options ?? []} label={widget.label ?? outputId}
                                    value={draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget)}
                                    disabled={false} sending={sendingId === widgetKey(binding.node_id, outputId)}
                                    onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v}
                                    onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))}
                                />
                            {:else if widget.type === "path" || widget.type === "file_picker" || widget.type === "directory"}
                                <ControlPath
                                    {outputId} xw={widget} mode={widget.type === "directory" ? "directory" : "file"}
                                    value={draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget)}
                                    disabled={false} sending={sendingId === widgetKey(binding.node_id, outputId)}
                                    onValueChange={(v) => draftValues[widgetKey(binding.node_id, outputId)] = v}
                                    onSend={() => handleEmit(binding.node_id, outputId, draftValues[widgetKey(binding.node_id, outputId)] ?? initialValue(binding, outputId, widget))}
                                />
                            {:else if widget.type === "file"}
                                <input type="file" class="text-sm cursor-pointer file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-primary file:text-primary-foreground hover:file:bg-primary/90" onchange={(e) => emitFile(binding.node_id, outputId, (e.currentTarget as HTMLInputElement).files)} />
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
