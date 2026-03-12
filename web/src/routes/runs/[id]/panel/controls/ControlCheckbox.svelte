<script lang="ts">
    import { Label } from "$lib/components/ui/label/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send } from "lucide-svelte";
    import type { NormalizedOption } from "../panel-utils";

    interface Props {
        outputId: string;
        options: NormalizedOption[];
        value: string[];
        label: string;
        disabled: boolean;
        sending: boolean;
        onSend: () => void;
        onValueChange: (v: string[]) => void;
    }

    let {
        outputId,
        options,
        value = [],
        label,
        disabled,
        sending,
        onSend,
        onValueChange,
    }: Props = $props();

    function toggleItem(optValue: string, checked: boolean) {
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

<div class="flex gap-2 border rounded-lg p-3 bg-muted/20">
    <div class="flex flex-wrap gap-x-4 gap-y-1.5 flex-1">
        {#each options as opt}
            <div class="flex items-center space-x-2">
                <input
                    type="checkbox"
                    id="widget-{outputId}-{opt.value}"
                    checked={(value || []).includes(opt.value)}
                    {disabled}
                    onchange={(e) =>
                        toggleItem(
                            opt.value,
                            (e.currentTarget as HTMLInputElement).checked,
                        )}
                    class="size-4 rounded"
                />
                <Label
                    for="widget-{outputId}-{opt.value}"
                    class="text-sm font-normal cursor-pointer"
                    >{opt.label}</Label
                >
            </div>
        {/each}
    </div>
    <Button
        size="icon"
        variant="ghost"
        class="shrink-0 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground self-end"
        disabled={disabled || sending || !value?.length}
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
