<script lang="ts">
    import { RefreshCw, Play } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import * as Select from "$lib/components/ui/select/index.js";

    let {
        methods = [],
        selectedMethod = $bindable(""),
        inputJson = $bindable("{}"),
        outputJson = "",
        invoking = false,
        onInvoke = () => {},
    } = $props<{
        methods: any[];
        selectedMethod: string;
        inputJson: string;
        outputJson?: string;
        invoking?: boolean;
        onInvoke?: () => void;
    }>();
</script>

<div class="grid h-full min-h-0 grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
    <div class="flex min-h-0 flex-col rounded-md border bg-card">
        <div class="flex items-center justify-between gap-3 border-b bg-muted/20 p-3">
            <Select.Root type="single" bind:value={selectedMethod}>
                <Select.Trigger class="w-56">
                    {selectedMethod || "Select method"}
                </Select.Trigger>
                <Select.Content>
                    {#each methods as method}
                        <Select.Item value={method.name}>{method.name}</Select.Item>
                    {/each}
                </Select.Content>
            </Select.Root>
            <Button
                class="gap-2"
                disabled={!selectedMethod || invoking}
                onclick={onInvoke}
            >
                {#if invoking}
                    <RefreshCw class="size-4 animate-spin" />
                {:else}
                    <Play class="size-4" />
                {/if}
                Invoke
            </Button>
        </div>
        <Textarea
            bind:value={inputJson}
            class="min-h-0 flex-1 resize-none rounded-none border-0 font-mono text-xs focus-visible:ring-0"
            spellcheck={false}
        />
    </div>

    <div class="flex min-h-0 flex-col rounded-md border bg-card">
        <div class="border-b bg-muted/20 p-3 text-sm font-medium">Result</div>
        <pre class="min-h-0 flex-1 overflow-auto p-4 text-xs font-mono whitespace-pre-wrap">{outputJson || "No invocation result yet."}</pre>
    </div>
</div>
