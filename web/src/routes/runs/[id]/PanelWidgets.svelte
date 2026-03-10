<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { post } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import { Slider } from "$lib/components/ui/slider/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
    import { PathPicker } from "$lib/components/ui/path-picker/index.js";
    import { Send } from "lucide-svelte";

    interface Props {
        runId: string;
        widgets: Record<string, any>;
        disabled?: boolean;
    }

    let { runId, widgets, disabled = false }: Props = $props();

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
                value: Array.isArray(value) ? value.join(',') : String(value),
            });
        } catch (e: any) {
            console.error(`Widget send failed for '${outputId}':`, e);
        } finally {
            sending = { ...sending, [outputId]: false };
        }
    }

    // Derive sorted widget entries
    let widgetEntries = $derived(Object.entries(widgets));

    // Resolve widget type from x-widget or schema type fallback
    function resolveWidgetType(def: any): string {
        return def?.["x-widget"]?.type || "input";
    }

    // ── Hotkey support ──

    function parseHotkey(hotkey: string) {
        const parts = hotkey.toLowerCase().split("+").map((p) => p.trim());
        return {
            ctrl: parts.includes("ctrl") || parts.includes("control"),
            meta: parts.includes("meta") || parts.includes("cmd"),
            alt: parts.includes("alt"),
            shift: parts.includes("shift"),
            key: parts.filter(
                (p) => !["ctrl", "control", "meta", "cmd", "alt", "shift"].includes(p),
            )[0] || "",
        };
    }

    function matchesHotkey(e: KeyboardEvent, parsed: ReturnType<typeof parseHotkey>): boolean {
        const ctrlOrMeta = parsed.ctrl || parsed.meta;
        if (ctrlOrMeta && !(e.ctrlKey || e.metaKey)) return false;
        if (!ctrlOrMeta && (e.ctrlKey || e.metaKey)) return false;
        if (parsed.alt !== e.altKey) return false;
        if (parsed.shift !== e.shiftKey) return false;
        return e.key.toLowerCase() === parsed.key;
    }

    // Display-friendly hotkey label (⌘/⌃/⇧/⌥ symbols)
    function formatHotkey(hotkey: string): string {
        const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);
        return hotkey
            .split("+")
            .map((p) => {
                const k = p.trim().toLowerCase();
                if (k === "ctrl" || k === "control") return isMac ? "⌃" : "Ctrl";
                if (k === "meta" || k === "cmd") return isMac ? "⌘" : "Ctrl";
                if (k === "alt") return isMac ? "⌥" : "Alt";
                if (k === "shift") return isMac ? "⇧" : "Shift";
                if (k === "enter") return "↵";
                if (k === "space") return "␣";
                if (k === "escape" || k === "esc") return "Esc";
                return p.trim().toUpperCase();
            })
            .join(isMac ? "" : "+");
    }

    function handleGlobalKeydown(e: KeyboardEvent) {
        if (disabled) return;
        // Skip if user is typing in an input/textarea
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
        {@const label = xw.label || def?.label || outputId}
        {@const isSending = sending[outputId] || false}
        {@const span = xw.span || 12}

        <div
            class="flex flex-col gap-1.5 w-full md:col-span-{span}"
            style="grid-column: span {span} / span {span};"
        >
            <label
                for="widget-{outputId}"
                class="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/80 pl-1"
            >
                {label}
            </label>

            <!-- Button (trigger action) -->
            {#if widgetType === "button"}
                <Button
                    variant={xw.variant || "default"}
                    class="w-full"
                    {disabled}
                    onclick={() => {
                        values[outputId] = xw.value || "clicked";
                        sendWidget(outputId);
                    }}
                >
                    {#if isSending}
                        <div
                            class="h-4 w-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2"
                        ></div>
                    {/if}
                    {label}
                    {#if xw.hotkey}
                        <kbd
                            class="ml-2 pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded border bg-muted/50 px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-70"
                        >
                            {formatHotkey(xw.hotkey)}
                        </kbd>
                    {/if}
                </Button>

            <!-- Input (default) -->
            {:else if widgetType === "input"}
                <div class="relative group w-full">
                    <input
                        id="widget-{outputId}"
                        type="text"
                        bind:value={values[outputId]}
                        placeholder={xw.placeholder || `Enter ${label}...`}
                        class="w-full h-10 pl-3 pr-10 rounded-lg border bg-background text-sm focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 transition-shadow"
                        {disabled}
                        onkeydown={(e) => {
                            if (e.key === "Enter") {
                                e.preventDefault();
                                sendWidget(outputId);
                            }
                        }}
                    />
                    <Button
                        size="icon"
                        variant="ghost"
                        class="absolute right-1 top-1 bottom-1 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
                        disabled={disabled ||
                            isSending ||
                            !values[outputId]?.toString().trim()}
                        onclick={() => sendWidget(outputId)}
                        title="Send {label}"
                    >
                        {#if isSending}
                            <div
                                class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"
                            ></div>
                        {:else}
                            <Send class="h-3.5 w-3.5" />
                        {/if}
                    </Button>
                </div>

                <!-- Textarea -->
            {:else if widgetType === "textarea"}
                <div class="relative group w-full">
                    <textarea
                        id="widget-{outputId}"
                        bind:value={values[outputId]}
                        placeholder={xw.placeholder || `Enter ${label}...`}
                        class="w-full min-h-[80px] pl-3 pr-10 py-2 rounded-lg border bg-background text-sm focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 transition-shadow resize-y"
                        {disabled}
                    ></textarea>
                    <Button
                        size="icon"
                        variant="ghost"
                        class="absolute right-1 bottom-1 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
                        disabled={disabled ||
                            isSending ||
                            !values[outputId]?.toString().trim()}
                        onclick={() => sendWidget(outputId)}
                        title="Send {label}"
                    >
                        {#if isSending}
                            <div
                                class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"
                            ></div>
                        {:else}
                            <Send class="h-3.5 w-3.5" />
                        {/if}
                    </Button>
                </div>

                <!-- Select (instant send) -->
            {:else if widgetType === "select"}
                <Select.Root
                    type="single"
                    value={values[outputId] ?? def?.default}
                    onValueChange={(v) => {
                        values[outputId] = v;
                        sendWidget(outputId);
                    }}
                >
                    <Select.Trigger class="w-full" {disabled}>
                        {values[outputId] ?? def?.default ?? "Select..."}
                    </Select.Trigger>
                    <Select.Content>
                        {#each xw.options || [] as opt}
                            <Select.Item value={String(opt)}>{opt}</Select.Item>
                        {/each}
                    </Select.Content>
                </Select.Root>

                <!-- Slider (instant send) -->
            {:else if widgetType === "slider"}
                <div class="flex items-center gap-4">
                    <Slider
                        type="single"
                        value={values[outputId] ?? def?.default ?? 0}
                        min={xw.min ?? 0}
                        max={xw.max ?? 100}
                        step={xw.step ?? 1}
                        {disabled}
                        onValueChange={(v) => {
                            values[outputId] = v;
                            sendWidget(outputId);
                        }}
                        class="flex-1"
                    />
                    <span
                        class="w-14 text-center text-sm font-mono tabular-nums text-muted-foreground"
                    >
                        {values[outputId] ?? def?.default ?? 0}
                    </span>
                </div>

                <!-- Switch (instant send) -->
            {:else if widgetType === "switch"}
                <div
                    class="flex items-center space-x-2 border rounded-lg p-3 bg-muted/20"
                >
                    <Switch
                        id="widget-{outputId}"
                        {disabled}
                        checked={!!values[outputId]}
                        onCheckedChange={(v) => {
                            values[outputId] = v;
                            sendWidget(outputId);
                        }}
                    />
                    <Label
                        for="widget-{outputId}"
                        class="font-normal cursor-pointer flex-1 text-sm"
                    >
                        {xw.switchLabel || `Enable`}
                    </Label>
                </div>

                <!-- Radio (with send button) -->
            {:else if widgetType === "radio"}
                <div class="flex gap-2 border rounded-lg p-3 bg-muted/20">
                    <RadioGroup.Root
                        value={String(values[outputId] ?? def?.default ?? "")}
                        {disabled}
                        onValueChange={(v) => {
                            const isNumber =
                                typeof (xw.options || [])[0] === "number";
                            values[outputId] = isNumber ? Number(v) : v;
                        }}
                        class="flex flex-wrap gap-x-4 gap-y-1.5 flex-1"
                    >
                        {#each xw.options || [] as opt}
                            <div class="flex items-center space-x-2">
                                <RadioGroup.Item
                                    value={String(opt)}
                                    id="widget-{outputId}-{opt}"
                                />
                                <Label
                                    for="widget-{outputId}-{opt}"
                                    class="text-sm font-normal">{opt}</Label
                                >
                            </div>
                        {/each}
                    </RadioGroup.Root>
                    <Button
                        size="icon"
                        variant="ghost"
                        class="shrink-0 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground self-end"
                        disabled={disabled ||
                            isSending ||
                            values[outputId] === undefined}
                        onclick={() => sendWidget(outputId)}
                        title="Send {label}"
                    >
                        {#if isSending}
                            <div
                                class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"
                            ></div>
                        {:else}
                            <Send class="h-3.5 w-3.5" />
                        {/if}
                    </Button>
                </div>

                <!-- Checkbox / multi-select (with send button) -->
            {:else if widgetType === "checkbox"}
                <div class="flex gap-2 border rounded-lg p-3 bg-muted/20">
                    <div class="flex flex-wrap gap-x-4 gap-y-1.5 flex-1">
                        {#each xw.options || [] as opt}
                            <div class="flex items-center space-x-2">
                                <input
                                    type="checkbox"
                                    id="widget-{outputId}-{opt}"
                                    checked={(values[outputId] || []).includes(String(opt))}
                                    {disabled}
                                    onchange={(e) => {
                                        const checked = (e.currentTarget as HTMLInputElement).checked;
                                        const current = Array.isArray(values[outputId]) ? [...values[outputId]] : [];
                                        const val = String(opt);
                                        if (checked && !current.includes(val)) {
                                            current.push(val);
                                        } else if (!checked) {
                                            const idx = current.indexOf(val);
                                            if (idx >= 0) current.splice(idx, 1);
                                        }
                                        values[outputId] = current;
                                    }}
                                    class="size-4 rounded"
                                />
                                <Label
                                    for="widget-{outputId}-{opt}"
                                    class="text-sm font-normal cursor-pointer">{opt}</Label
                                >
                            </div>
                        {/each}
                    </div>
                    <Button
                        size="icon"
                        variant="ghost"
                        class="shrink-0 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground self-end"
                        disabled={disabled ||
                            isSending ||
                            !values[outputId]?.length}
                        onclick={() => sendWidget(outputId)}
                        title="Send {label}"
                    >
                        {#if isSending}
                            <div
                                class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"
                            ></div>
                        {:else}
                            <Send class="h-3.5 w-3.5" />
                        {/if}
                    </Button>
                </div>

                <!-- File / Directory path (with send button via PathPicker) -->
            {:else if widgetType === "file" || widgetType === "directory"}
                <PathPicker
                    mode={widgetType}
                    id="widget-{outputId}"
                    bind:value={values[outputId]}
                    placeholder={xw.placeholder || undefined}
                    {disabled}
                    showConfirmBtn={true}
                    confirming={isSending}
                    onConfirm={() => sendWidget(outputId)}
                />

                <!-- Fallback: treat as input -->
            {:else}
                <div class="relative group w-full">
                    <input
                        id="widget-{outputId}"
                        type="text"
                        bind:value={values[outputId]}
                        placeholder={xw.placeholder || `Enter ${label}...`}
                        class="w-full h-10 pl-3 pr-10 rounded-lg border bg-background text-sm focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 transition-shadow"
                        {disabled}
                        onkeydown={(e) => {
                            if (e.key === "Enter") {
                                e.preventDefault();
                                sendWidget(outputId);
                            }
                        }}
                    />
                    <Button
                        size="icon"
                        variant="ghost"
                        class="absolute right-1 top-1 bottom-1 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
                        disabled={disabled ||
                            isSending ||
                            !values[outputId]?.toString().trim()}
                        onclick={() => sendWidget(outputId)}
                        title="Send {label}"
                    >
                        {#if isSending}
                            <div
                                class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"
                            ></div>
                        {:else}
                            <Send class="h-3.5 w-3.5" />
                        {/if}
                    </Button>
                </div>
            {/if}
        </div>
    {/each}
</div>
