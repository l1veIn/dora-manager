<script lang="ts">
    import { post } from "$lib/api";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send, Plus, Trash2 } from "lucide-svelte";

    let { runId } = $props<{
        runId: string;
    }>();

    // Default common outputs or dynamically populated
    // Future: fetch known dataflow outputs from dm-config or user setup
    let customOutputs = $state<
        Array<{ id: string; value: string; sending: boolean }>
    >([{ id: "speed", value: "0.5", sending: false }]);

    let newOutputId = $state("");

    async function handleSend(index: number) {
        const item = customOutputs[index];
        if (!item.id || !item.value) return;

        customOutputs[index].sending = true;
        try {
            await post(`/panel/${runId}/commands`, {
                output_id: item.id,
                value: item.value,
            });
            // Optionally clear value on success if treating as one-shot commands
        } catch (e) {
            console.error("Failed to send command", e);
        } finally {
            customOutputs[index].sending = false;
        }
    }

    function addControl() {
        if (!newOutputId) return;
        if (customOutputs.find((c) => c.id === newOutputId)) return;

        customOutputs = [
            ...customOutputs,
            { id: newOutputId, value: "", sending: false },
        ];
        newOutputId = "";
    }

    function removeControl(index: number) {
        customOutputs = customOutputs.filter((_, i) => i !== index);
    }
</script>

<div class="flex flex-col gap-3">
    <div class="flex items-center justify-between pb-1">
        <h3 class="text-sm font-semibold flex items-center gap-2">
            Output Controls
        </h3>
        <span class="text-[10px] uppercase text-muted-foreground"
            >inject() via index.db</span
        >
    </div>

    <!-- Generated Controls -->
    <div
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3"
    >
        {#each customOutputs as control, i}
            <div
                class="flex items-center gap-2 bg-muted/30 border rounded-md p-2"
            >
                <div class="flex-col gap-1 w-full min-w-0 flex">
                    <label
                        for={`control-${i}`}
                        class="text-[10px] font-mono text-muted-foreground uppercase flex justify-between"
                    >
                        <span>{control.id}</span>
                        <button
                            class="hover:text-destructive transition-colors px-1"
                            onclick={(e: MouseEvent) => {
                                e.preventDefault();
                                removeControl(i);
                            }}
                        >
                            <Trash2 class="size-3" />
                        </button>
                    </label>
                    <div class="flex gap-1">
                        <Input
                            id={`control-${i}`}
                            type="text"
                            bind:value={control.value}
                            disabled={control.sending}
                            class="h-7 text-xs font-mono bg-background"
                            placeholder="Data (JSON str or val)"
                            onkeydown={(e: KeyboardEvent) =>
                                e.key === "Enter" && handleSend(i)}
                        />
                        <Button
                            size="sm"
                            variant="default"
                            class="h-7 w-10 px-0 shrink-0"
                            disabled={control.sending || !control.value}
                            onclick={() => handleSend(i)}
                        >
                            <Send
                                class="size-3 {control.sending
                                    ? 'animate-pulse'
                                    : ''}"
                            />
                        </Button>
                    </div>
                </div>
            </div>
        {/each}

        <!-- Add New Control -->
        <div
            class="flex items-center gap-2 bg-muted/10 border border-dashed rounded-md p-2 hover:bg-muted/30 transition-colors focus-within:bg-muted/30 focus-within:border-primary/50"
        >
            <div class="flex-col gap-1 w-full flex">
                <label
                    for="new-output"
                    class="text-[10px] text-muted-foreground uppercase"
                    >Add Output</label
                >
                <div class="flex gap-1">
                    <Input
                        id="new-output"
                        type="text"
                        bind:value={newOutputId}
                        class="h-7 text-xs font-mono"
                        placeholder="output_id"
                        onkeydown={(e: KeyboardEvent) =>
                            e.key === "Enter" && addControl()}
                    />
                    <Button
                        size="sm"
                        variant="ghost"
                        class="h-7 w-7 px-0 shrink-0 border"
                        disabled={!newOutputId}
                        onclick={addControl}
                    >
                        <Plus class="size-3" />
                    </Button>
                </div>
            </div>
        </div>
    </div>
</div>
