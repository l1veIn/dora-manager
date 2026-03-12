<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { post } from "$lib/api";
    import {
        smartResolve,
        resolveDefault,
        resolveOptions,
        parseHotkey,
        matchesHotkey,
    } from "./panel/panel-utils";

    import ControlButton from "./panel/controls/ControlButton.svelte";
    import ControlInput from "./panel/controls/ControlInput.svelte";
    import ControlTextarea from "./panel/controls/ControlTextarea.svelte";
    import ControlSelect from "./panel/controls/ControlSelect.svelte";
    import ControlSlider from "./panel/controls/ControlSlider.svelte";
    import ControlSwitch from "./panel/controls/ControlSwitch.svelte";
    import ControlRadio from "./panel/controls/ControlRadio.svelte";
    import ControlCheckbox from "./panel/controls/ControlCheckbox.svelte";
    import ControlPath from "./panel/controls/ControlPath.svelte";

    interface Props {
        runId: string;
        widgets: Record<string, any>;
        disabled?: boolean;
        latestAssets?: Record<string, any>;
    }

    let { runId, widgets, disabled = false, latestAssets = {} }: Props =
        $props();

    // Track current values for each widget
    let values = $state<Record<string, any>>({});
    let sending = $state<Record<string, boolean>>({});

    // Handle dynamically added widgets
    $effect(() => {
        const init: Record<string, any> = {};
        for (const [id, def] of Object.entries(widgets)) {
            if (!(id in values)) {
                init[id] = def?.default ?? "";
            }
        }
        if (Object.keys(init).length > 0) {
            values = { ...values, ...init };
        }
    });

    async function sendWidget(outputId: string) {
        const value = values[outputId];
        if (value === undefined && value !== false) return;
        sending = { ...sending, [outputId]: true };
        try {
            await post(`/runs/${runId}/panel/commands`, {
                output_id: outputId,
                value: Array.isArray(value) ? value.join(",") : String(value),
            });
        } catch (e: any) {
            console.error(`Widget send failed for '${outputId}':`, e);
        } finally {
            sending = { ...sending, [outputId]: false };
        }
    }

    let widgetEntries = $derived(Object.entries(widgets));

    function resolveWidgetType(def: any): string {
        return def?.["x-widget"]?.type || "input";
    }

    // Auto-select default when dynamic options arrive
    $effect(() => {
        for (const [outputId, def] of Object.entries(widgets)) {
            const xw = def?.["x-widget"];
            if (!xw?.bind || typeof xw.bind !== "string") continue;
            const overrides = smartResolve(xw, latestAssets);
            const opts = overrides.options;
            if (!opts || opts.length === 0) continue;
            const current = values[outputId];
            if (
                current == null ||
                current === "" ||
                !opts.some((o) => o.value === String(current))
            ) {
                const newDefault = resolveDefault(def, opts, latestAssets);
                if (newDefault != null) {
                    values[outputId] = newDefault;
                }
            }
        }
    });

    // ── Hotkey support ──

    function handleGlobalKeydown(e: KeyboardEvent) {
        if (disabled) return;
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

        for (const [outputId, def] of Object.entries(widgets)) {
            const xw = def?.["x-widget"];
            if (xw?.type !== "button" || !xw?.hotkey) continue;

            const parsed = parseHotkey(xw.hotkey);
            if (matchesHotkey(e, parsed)) {
                e.preventDefault();
                e.stopPropagation();
                values[outputId] = xw.value || "clicked";
                sendWidget(outputId);
                return;
            }
        }
    }

    onMount(() => {
        window.addEventListener("keydown", handleGlobalKeydown);
    });

    onDestroy(() => {
        window.removeEventListener("keydown", handleGlobalKeydown);
    });
</script>

<div class="grid grid-cols-1 md:grid-cols-12 gap-x-4 gap-y-4 w-full">
    {#each widgetEntries as [outputId, def]}
        {@const xw = def?.["x-widget"] || {}}
        {@const widgetType = resolveWidgetType(def)}
        {@const overrides = smartResolve(xw, latestAssets)}
        {@const label = overrides.label ?? xw.label ?? def?.label ?? outputId}
        {@const isSending = sending[outputId] || false}
        {@const span = xw.span || 12}
        {@const isDisabled = disabled || overrides.disabled === true}
        {@const isLoading = overrides.loading === true}
        {@const progress = typeof overrides.progress === 'number' ? Math.min(1, Math.max(0, overrides.progress)) : undefined}

        <div
            class="flex flex-col gap-1.5 w-full md:col-span-{span} relative"
            style="grid-column: span {span} / span {span};"
        >
            <label
                for="widget-{outputId}"
                class="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/80 pl-1 flex items-center gap-1.5"
            >
                {label}
                {#if isLoading}
                    <div class="h-3 w-3 border-[1.5px] border-primary border-t-transparent rounded-full animate-spin"></div>
                {/if}
            </label>

            {#if widgetType === "button"}
                <ControlButton
                    {outputId}
                    {xw}
                    {overrides}
                    {label}
                    disabled={isDisabled}
                    sending={isSending}
                    onSend={(v) => {
                        values[outputId] = v;
                        sendWidget(outputId);
                    }}
                />
            {:else if widgetType === "input"}
                <ControlInput
                    {outputId}
                    {xw}
                    {label}
                    bind:value={values[outputId]}
                    disabled={isDisabled}
                    sending={isSending}
                    onSend={() => sendWidget(outputId)}
                    onValueChange={(v) => (values[outputId] = v)}
                />
            {:else if widgetType === "textarea"}
                <ControlTextarea
                    {outputId}
                    {xw}
                    {label}
                    bind:value={values[outputId]}
                    disabled={isDisabled}
                    sending={isSending}
                    onSend={() => sendWidget(outputId)}
                    onValueChange={(v) => (values[outputId] = v)}
                />
            {:else if widgetType === "select"}
                <ControlSelect
                    {outputId}
                    options={resolveOptions(xw, overrides)}
                    value={values[outputId]}
                    defaultValue={def?.default}
                    disabled={isDisabled}
                    onValueChange={(v) => {
                        values[outputId] = v;
                        sendWidget(outputId);
                    }}
                />
            {:else if widgetType === "slider"}
                <ControlSlider
                    {outputId}
                    {xw}
                    value={values[outputId]}
                    defaultValue={def?.default}
                    disabled={isDisabled}
                    overrideValue={overrides.value}
                    onValueChange={(v) => {
                        values[outputId] = v;
                        sendWidget(outputId);
                    }}
                />
            {:else if widgetType === "switch"}
                <ControlSwitch
                    {outputId}
                    {xw}
                    value={values[outputId]}
                    disabled={isDisabled}
                    onValueChange={(v) => {
                        values[outputId] = v;
                        sendWidget(outputId);
                    }}
                />
            {:else if widgetType === "radio"}
                <ControlRadio
                    {outputId}
                    options={resolveOptions(xw, overrides)}
                    value={values[outputId]}
                    defaultValue={def?.default}
                    {label}
                    disabled={isDisabled}
                    sending={isSending}
                    onSend={() => sendWidget(outputId)}
                    onValueChange={(v) => (values[outputId] = v)}
                />
            {:else if widgetType === "checkbox"}
                <ControlCheckbox
                    {outputId}
                    options={resolveOptions(xw, overrides)}
                    value={values[outputId]}
                    {label}
                    disabled={isDisabled}
                    sending={isSending}
                    onSend={() => sendWidget(outputId)}
                    onValueChange={(v) => (values[outputId] = v)}
                />
            {:else if widgetType === "file" || widgetType === "directory"}
                <ControlPath
                    {outputId}
                    {xw}
                    mode={widgetType}
                    bind:value={values[outputId]}
                    disabled={isDisabled}
                    sending={isSending}
                    onSend={() => sendWidget(outputId)}
                    onValueChange={(v) => (values[outputId] = v)}
                />
            {:else}
                <ControlInput
                    {outputId}
                    {xw}
                    {label}
                    bind:value={values[outputId]}
                    disabled={isDisabled}
                    sending={isSending}
                    onSend={() => sendWidget(outputId)}
                    onValueChange={(v) => (values[outputId] = v)}
                />
            {/if}

            {#if isLoading && progress !== undefined}
                <div class="w-full h-1.5 rounded-full bg-muted overflow-hidden">
                    <div
                        class="h-full rounded-full bg-gradient-to-r from-primary/80 to-primary transition-all duration-300 ease-out"
                        style="width: {progress * 100}%"
                    ></div>
                </div>
            {/if}
        </div>
    {/each}
</div>
