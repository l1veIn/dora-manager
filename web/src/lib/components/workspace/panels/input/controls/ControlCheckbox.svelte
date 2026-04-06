<script lang="ts">
    import { Label } from "$lib/components/ui/label/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send } from "lucide-svelte";

    interface Props {
        outputId: string;
        options: any[];
        value: any[];
        label: string;
        disabled?: boolean;
        sending?: boolean;
        onSend: () => void;
        onValueChange: (v: any[]) => void;
    }

    let { outputId, options, value = [], label, disabled, sending, onSend, onValueChange }: Props = $props();

    function optionValue(opt: any) { return typeof opt === "object" ? opt.value : opt; }
    function optionLabel(opt: any) { return typeof opt === "object" ? (opt.label ?? opt.value) : opt; }

    function toggleItem(optValue: any, checked: boolean) {
        const current = Array.isArray(value) ? [...value] : [];
        if (checked && !current.includes(optValue)) {
            current.push(optValue);
        } else if (!checked) {
            const idx = current.indexOf(optValue);
            if (idx >= 0) current.splice(idx, 1);
        }
        onValueChange(current);
    }
</script>

<div class="flex gap-2 border shadow-sm rounded-lg p-3 bg-muted/20">
    <div class="flex flex-wrap gap-x-4 gap-y-1.5 flex-1">
        {#each options as opt}
            <div class="flex items-center space-x-2">
                <input
                    type="checkbox"
                    id="widget-{outputId}-{optionValue(opt)}"
                    checked={(value || []).includes(optionValue(opt))}
                    {disabled}
                    onchange={(e) => toggleItem(optionValue(opt), (e.currentTarget as HTMLInputElement).checked)}
                    class="size-4 rounded border-primary bg-background accent-primary"
                />
                <Label for="widget-{outputId}-{optionValue(opt)}" class="text-sm font-normal cursor-pointer">{optionLabel(opt)}</Label>
            </div>
        {/each}
    </div>
    <Button
        size="icon" variant="ghost"
        class="shrink-0 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground self-end bg-background border shadow-sm"
        disabled={disabled || sending || !value?.length}
        onclick={() => onSend()} title="Send {label}"
    >
        {#if sending}
            <div class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
        {:else}
            <Send class="h-3.5 w-3.5" />
        {/if}
    </Button>
</div>
