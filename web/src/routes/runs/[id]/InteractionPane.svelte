<script lang="ts">
    import { marked } from "marked";
    import DOMPurify from "dompurify";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";

    type DisplayEntry = {
        node_id: string;
        label: string;
        kind: string;
        file?: string | null;
        content?: any;
        render: string;
        tail: boolean;
        max_lines: number;
        updated_at: number;
    };

    type InputBinding = {
        node_id: string;
        label: string;
        widgets: Record<string, any>;
        current_values: Record<string, any>;
        updated_at: number;
    };

    let {
        runId,
        displays = [],
        inputs = [],
        onEmit,
    } = $props<{
        runId: string;
        displays: DisplayEntry[];
        inputs: InputBinding[];
        onEmit: (nodeId: string, outputId: string, value: any) => Promise<void>;
    }>();

    let textContent = $state<Record<string, string>>({});
    let loadingKeys = $state<Record<string, boolean>>({});
    let draftValues = $state<Record<string, any>>({});

    function displayUrl(file: string | null | undefined) {
        if (!file) return "";
        return `/api/runs/${runId}/artifacts/${file}`;
    }

    function inlineDisplayText(entry: DisplayEntry) {
        if (entry.render === "json") {
            return JSON.stringify(entry.content ?? null, null, 2);
        }
        if (typeof entry.content === "string") return entry.content;
        if (entry.content == null) return "";
        return JSON.stringify(entry.content, null, 2);
    }

    function widgetKey(nodeId: string, outputId: string) {
        return `${nodeId}:${outputId}`;
    }

    function optionValue(option: any) {
        return typeof option === "object" ? option.value : option;
    }

    function optionLabel(option: any) {
        return typeof option === "object" ? option.label ?? option.value : option;
    }

    function widgetEntries(binding: InputBinding): Array<[string, any]> {
        return Object.entries(binding.widgets ?? {}) as Array<[string, any]>;
    }

    function initialValue(binding: InputBinding, outputId: string, widget: any) {
        const key = widgetKey(binding.node_id, outputId);
        if (draftValues[key] !== undefined) return draftValues[key];
        if (binding.current_values?.[outputId] !== undefined) return binding.current_values[outputId];
        if (widget?.default !== undefined) return widget.default;
        if (widget?.type === "checkbox") return [];
        if (widget?.type === "switch") return false;
        if (widget?.type === "slider") return widget?.min ?? 0;
        return "";
    }

    async function loadDisplay(entry: DisplayEntry) {
        if (entry.kind === "inline") {
            const key = entry.node_id;
            if (entry.render === "markdown") {
                textContent[key] = DOMPurify.sanitize(marked.parse(String(entry.content ?? "")) as string);
            } else if (entry.render === "json") {
                textContent[key] = JSON.stringify(entry.content ?? null, null, 2);
            } else if (entry.tail && typeof entry.content === "string") {
                const lines = entry.content.split("\n");
                textContent[key] = lines.slice(-entry.max_lines).join("\n");
            } else {
                textContent[key] = inlineDisplayText(entry);
            }
            loadingKeys[key] = false;
            return;
        }
        if (!["text", "json", "markdown"].includes(entry.render)) return;
        const key = entry.node_id;
        loadingKeys[key] = true;
        try {
            const res = await fetch(displayUrl(entry.file));
            if (!res.ok) throw new Error(await res.text());
            const raw = await res.text();
            if (entry.render === "json") {
                try {
                    textContent[key] = JSON.stringify(JSON.parse(raw), null, 2);
                } catch {
                    textContent[key] = raw;
                }
            } else if (entry.render === "markdown") {
                textContent[key] = DOMPurify.sanitize(marked.parse(raw) as string);
            } else if (entry.tail) {
                const lines = raw.split("\n");
                textContent[key] = lines.slice(-entry.max_lines).join("\n");
            } else {
                textContent[key] = raw;
            }
        } catch (err: any) {
            textContent[key] = `Failed to load artifact: ${err.message ?? err}`;
        } finally {
            loadingKeys[key] = false;
        }
    }

    $effect(() => {
        for (const entry of displays) {
            loadDisplay(entry);
        }
    });

    async function emit(nodeId: string, outputId: string, value: any) {
        draftValues[widgetKey(nodeId, outputId)] = value;
        await onEmit(nodeId, outputId, value);
    }

    async function emitFile(nodeId: string, outputId: string, fileList: FileList | null) {
        const file = fileList?.[0];
        if (!file) return;
        const buf = await file.arrayBuffer();
        const bytes = new Uint8Array(buf);
        let binary = "";
        for (const byte of bytes) binary += String.fromCharCode(byte);
        await emit(nodeId, outputId, btoa(binary));
    }

    function toggleCheckbox(binding: InputBinding, outputId: string, option: any, checked: boolean) {
        const current = [...(initialValue(binding, outputId, binding.widgets[outputId]) ?? [])];
        const value = optionValue(option);
        const next = checked ? [...current.filter((item) => item !== value), value] : current.filter((item) => item !== value);
        emit(binding.node_id, outputId, next);
    }
</script>

<div class="flex flex-col h-full min-h-0 overflow-hidden bg-card border-b">
    <div class="px-4 py-3 border-b bg-muted/20 flex items-center justify-between">
        <div>
            <h2 class="text-sm font-semibold tracking-tight">Interaction</h2>
            <p class="text-xs text-muted-foreground">Display nodes and input nodes bridged through dm-server</p>
        </div>
        <div class="flex items-center gap-2">
            {#if displays.length > 0}
                <Badge variant="secondary">{displays.length} display</Badge>
            {/if}
            {#if inputs.length > 0}
                <Badge variant="secondary">{inputs.length} input</Badge>
            {/if}
        </div>
    </div>

    <div class="grid md:grid-cols-2 gap-4 p-4 overflow-auto">
        <section class="space-y-4">
            <div class="flex items-center justify-between">
                <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Displays</h3>
            </div>
            {#if displays.length === 0}
                <div class="rounded-md border border-dashed p-4 text-sm text-muted-foreground">No display nodes registered for this run.</div>
            {:else}
                {#each displays as entry}
                    <div class="rounded-lg border bg-background overflow-hidden">
                        <div class="px-3 py-2 border-b flex items-center justify-between">
                            <div>
                                <div class="font-medium text-sm">{entry.label}</div>
                                <div class="text-[11px] text-muted-foreground font-mono">{entry.kind === "inline" ? "<inline>" : entry.file}</div>
                            </div>
                            <Badge variant="outline" class="uppercase text-[10px]">{entry.render}</Badge>
                        </div>
                        <div class="p-3">
                            {#if entry.render === "image"}
                                <img src={displayUrl(entry.file)} alt={entry.label} class="max-h-96 w-full object-contain rounded border bg-black/5" />
                            {:else if entry.render === "audio"}
                                <audio controls class="w-full" src={displayUrl(entry.file)}></audio>
                            {:else if entry.render === "video"}
                                <video controls class="w-full rounded border bg-black" src={displayUrl(entry.file)}>
                                    <track kind="captions" />
                                </video>
                            {:else if entry.render === "markdown"}
                                <div class="prose prose-sm max-w-none dark:prose-invert">
                                    {@html textContent[entry.node_id] ?? ""}
                                </div>
                            {:else}
                                <pre class="text-xs font-mono whitespace-pre-wrap break-all bg-muted/30 rounded-md p-3 min-h-20 overflow-auto">{loadingKeys[entry.node_id] ? "Loading..." : textContent[entry.node_id] ?? ""}</pre>
                            {/if}
                        </div>
                    </div>
                {/each}
            {/if}
        </section>

        <section class="space-y-4">
            <div class="flex items-center justify-between">
                <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Inputs</h3>
            </div>
            {#if inputs.length === 0}
                <div class="rounded-md border border-dashed p-4 text-sm text-muted-foreground">No input nodes registered for this run.</div>
            {:else}
                {#each inputs as binding}
                    <div class="rounded-lg border bg-background overflow-hidden">
                        <div class="px-3 py-2 border-b">
                            <div class="font-medium text-sm">{binding.label}</div>
                            <div class="text-[11px] text-muted-foreground font-mono">{binding.node_id}</div>
                        </div>
                        <div class="p-3 space-y-4">
                            {#each widgetEntries(binding) as [outputId, widget]}
                                <div class="space-y-2">
                                    <div class="text-xs font-medium text-muted-foreground uppercase tracking-wider">{widget.label ?? outputId}</div>
                                    {#if widget.type === "textarea"}
                                        <textarea
                                            class="w-full min-h-24 rounded-md border bg-background px-3 py-2 text-sm"
                                            value={initialValue(binding, outputId, widget)}
                                            placeholder={widget.placeholder ?? ""}
                                            onblur={(e) => emit(binding.node_id, outputId, (e.currentTarget as HTMLTextAreaElement).value)}
                                        ></textarea>
                                    {:else if widget.type === "input"}
                                        <Input
                                            value={initialValue(binding, outputId, widget)}
                                            placeholder={widget.placeholder ?? ""}
                                            onblur={(e) => emit(binding.node_id, outputId, (e.currentTarget as HTMLInputElement).value)}
                                        />
                                    {:else if widget.type === "button"}
                                        <Button onclick={() => emit(binding.node_id, outputId, widget.value ?? widget.label ?? outputId)}>
                                            {widget.label ?? outputId}
                                        </Button>
                                    {:else if widget.type === "select"}
                                        <select
                                            class="w-full rounded-md border bg-background px-3 py-2 text-sm"
                                            value={initialValue(binding, outputId, widget)}
                                            onchange={(e) => emit(binding.node_id, outputId, (e.currentTarget as HTMLSelectElement).value)}
                                        >
                                            {#each widget.options ?? [] as option}
                                                <option value={optionValue(option)}>{optionLabel(option)}</option>
                                            {/each}
                                        </select>
                                    {:else if widget.type === "slider"}
                                        <div class="space-y-2">
                                            <input
                                                class="w-full"
                                                type="range"
                                                min={widget.min ?? 0}
                                                max={widget.max ?? 100}
                                                step={widget.step ?? 1}
                                                value={initialValue(binding, outputId, widget)}
                                                onchange={(e) => emit(binding.node_id, outputId, Number((e.currentTarget as HTMLInputElement).value))}
                                            />
                                            <div class="text-xs text-muted-foreground font-mono">{initialValue(binding, outputId, widget)}</div>
                                        </div>
                                    {:else if widget.type === "switch"}
                                        <label class="flex items-center gap-2 text-sm">
                                            <input
                                                type="checkbox"
                                                checked={Boolean(initialValue(binding, outputId, widget))}
                                                onchange={(e) => emit(binding.node_id, outputId, (e.currentTarget as HTMLInputElement).checked)}
                                            />
                                            <span>{widget.switchLabel ?? widget.label ?? outputId}</span>
                                        </label>
                                    {:else if widget.type === "radio"}
                                        <div class="space-y-2">
                                            {#each widget.options ?? [] as option}
                                                <label class="flex items-center gap-2 text-sm">
                                                    <input
                                                        type="radio"
                                                        name={widgetKey(binding.node_id, outputId)}
                                                        value={optionValue(option)}
                                                        checked={initialValue(binding, outputId, widget) === optionValue(option)}
                                                        onchange={() => emit(binding.node_id, outputId, optionValue(option))}
                                                    />
                                                    <span>{optionLabel(option)}</span>
                                                </label>
                                            {/each}
                                        </div>
                                    {:else if widget.type === "checkbox"}
                                        <div class="space-y-2">
                                            {#each widget.options ?? [] as option}
                                                <label class="flex items-center gap-2 text-sm">
                                                    <input
                                                        type="checkbox"
                                                        checked={(initialValue(binding, outputId, widget) ?? []).includes(optionValue(option))}
                                                        onchange={(e) => toggleCheckbox(binding, outputId, option, (e.currentTarget as HTMLInputElement).checked)}
                                                    />
                                                    <span>{optionLabel(option)}</span>
                                                </label>
                                            {/each}
                                        </div>
                                    {:else if widget.type === "file"}
                                        <input type="file" onchange={(e) => emitFile(binding.node_id, outputId, (e.currentTarget as HTMLInputElement).files)} />
                                    {:else}
                                        <div class="rounded-md border border-dashed p-3 text-sm text-muted-foreground">Unsupported widget type: {widget.type ?? "unknown"}</div>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    </div>
                {/each}
            {/if}
        </section>
    </div>
</div>
