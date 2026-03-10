<script lang="ts">
    import { post } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send } from "lucide-svelte";

    interface Props {
        runId: string;
        widgets: Record<string, any>;
        disabled?: boolean;
    }

    let { runId, widgets, disabled = false }: Props = $props();

    // Track current values for each widget
    let values = $state<Record<string, string>>({});
    let sending = $state<Record<string, boolean>>({});

    // Initialize values from widget defaults
    $effect(() => {
        const init: Record<string, string> = {};
        for (const [id, def] of Object.entries(widgets)) {
            if (def?.default !== undefined && !(id in values)) {
                init[id] = String(def.default);
            }
        }
        if (Object.keys(init).length > 0) {
            values = { ...values, ...init };
        }
    });

    async function sendWidget(outputId: string) {
        const value = values[outputId];
        if (value === undefined) return;
        sending = { ...sending, [outputId]: true };
        try {
            await post(`/runs/${runId}/panel/commands`, {
                output_id: outputId,
                value,
            });
        } catch (e: any) {
            console.error(`Widget send failed for '${outputId}':`, e);
        } finally {
            sending = { ...sending, [outputId]: false };
        }
    }

    function handleKeydown(e: KeyboardEvent, outputId: string) {
        if (e.key === "Enter") {
            e.preventDefault();
            sendWidget(outputId);
        }
    }

    // Derive sorted widget entries
    let widgetEntries = $derived(Object.entries(widgets));
</script>

<div class="grid grid-cols-1 md:grid-cols-12 gap-x-4 gap-y-4 w-full">
    {#each widgetEntries as [outputId, def]}
        {@const label = def?.["x-widget"]?.label || def?.label || outputId}
        {@const placeholder =
            def?.["x-widget"]?.placeholder || `Enter ${label}...`}
        {@const isSending = sending[outputId] || false}
        {@const span = def?.["x-widget"]?.span || 12}

        <div class="flex flex-col gap-1.5 w-full md:col-span-{span}" style="grid-column: span {span} / span {span};">
            <label
                for="widget-{outputId}"
                class="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/80 pl-1"
            >
                {label}
            </label>
            <div class="relative group w-full">
                <input
                    id="widget-{outputId}"
                    type="text"
                    bind:value={values[outputId]}
                    {placeholder}
                    class="w-full h-10 pl-3 pr-10 rounded-lg border bg-background text-sm focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 transition-shadow"
                    {disabled}
                    onkeydown={(e) => {
                        if (e.key === 'Enter') {
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
                        !values[outputId]?.trim()}
                    onclick={() => sendWidget(outputId)}
                    title="Send {label}"
                >
                    {#if isSending}
                        <div class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
                    {:else}
                        <Send class="h-3.5 w-3.5" />
                    {/if}
                </Button>
            </div>
        </div>
    {/each}
</div>
