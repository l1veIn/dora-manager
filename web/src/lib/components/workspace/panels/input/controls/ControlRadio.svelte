<script lang="ts">
    import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send } from "lucide-svelte";

    interface Props {
        outputId: string;
        options: any[];
        value: any;
        defaultValue: any;
        label: string;
        disabled?: boolean;
        sending?: boolean;
        onSend: () => void;
        onValueChange: (v: string) => void;
    }

    let { outputId, options, value, defaultValue, label, disabled, sending, onSend, onValueChange }: Props = $props();

    function optionValue(opt: any) { return typeof opt === "object" ? opt.value : opt; }
    function optionLabel(opt: any) { return typeof opt === "object" ? (opt.label ?? opt.value) : opt; }
</script>

<div class="flex gap-2 border shadow-sm rounded-lg p-3 bg-muted/20">
    <RadioGroup.Root
        value={String(value ?? defaultValue ?? "")}
        {disabled}
        onValueChange={(v) => onValueChange(v)}
        class="flex flex-wrap gap-x-4 gap-y-1.5 flex-1"
    >
        {#each options as opt}
            <div class="flex items-center space-x-2">
                <RadioGroup.Item value={String(optionValue(opt))} id="widget-{outputId}-{optionValue(opt)}" />
                <Label for="widget-{outputId}-{optionValue(opt)}" class="text-sm font-normal cursor-pointer">{optionLabel(opt)}</Label>
            </div>
        {/each}
    </RadioGroup.Root>
    <Button
        size="icon" variant="ghost" class="shrink-0 h-8 w-8 rounded-md bg-background border shadow-sm text-muted-foreground hover:text-foreground self-end"
        disabled={disabled || sending || value === undefined}
        onclick={() => onSend()} title="Send {label}"
    >
        {#if sending}
            <div class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
        {:else}
            <Send class="h-3.5 w-3.5" />
        {/if}
    </Button>
</div>
