<script lang="ts">
    import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send } from "lucide-svelte";
    import type { NormalizedOption } from "../panel-utils";

    interface Props {
        outputId: string;
        options: NormalizedOption[];
        value: any;
        defaultValue: any;
        label: string;
        disabled: boolean;
        sending: boolean;
        onSend: () => void;
        onValueChange: (v: string) => void;
    }

    let {
        outputId,
        options,
        value,
        defaultValue,
        label,
        disabled,
        sending,
        onSend,
        onValueChange,
    }: Props = $props();
</script>

<div class="flex gap-2 border rounded-lg p-3 bg-muted/20">
    <RadioGroup.Root
        value={String(value ?? defaultValue ?? "")}
        {disabled}
        onValueChange={(v) => onValueChange(v)}
        class="flex flex-wrap gap-x-4 gap-y-1.5 flex-1"
    >
        {#each options as opt}
            <div class="flex items-center space-x-2">
                <RadioGroup.Item
                    value={opt.value}
                    id="widget-{outputId}-{opt.value}"
                />
                <Label
                    for="widget-{outputId}-{opt.value}"
                    class="text-sm font-normal">{opt.label}</Label
                >
            </div>
        {/each}
    </RadioGroup.Root>
    <Button
        size="icon"
        variant="ghost"
        class="shrink-0 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground self-end"
        disabled={disabled || sending || value === undefined}
        onclick={() => onSend()}
        title="Send {label}"
    >
        {#if sending}
            <div
                class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"
            ></div>
        {:else}
            <Send class="h-3.5 w-3.5" />
        {/if}
    </Button>
</div>
