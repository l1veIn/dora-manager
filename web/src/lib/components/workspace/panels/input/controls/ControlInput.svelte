<script lang="ts">
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send } from "lucide-svelte";

    interface Props {
        outputId: string;
        xw: any;
        label: string;
        value: string;
        disabled?: boolean;
        sending?: boolean;
        onSend: () => void;
        onValueChange: (v: string) => void;
    }

    let { outputId, xw, label, value = $bindable(), disabled, sending, onSend, onValueChange }: Props = $props();
</script>

<div class="relative group w-full">
    <Input
        id="widget-{outputId}"
        bind:value
        placeholder={xw.placeholder || `Enter ${label}...`}
        class="w-full pr-10 rounded-lg bg-background shadow-sm"
        {disabled}
        oninput={() => onValueChange(value)}
        onkeydown={(e: KeyboardEvent) => {
            if (e.key === "Enter" && !e.shiftKey && value) {
                e.preventDefault();
                onSend();
            }
        }}
    />
    <Button
        size="icon" variant="ghost" class="absolute right-1 top-1 h-8 w-8 text-muted-foreground hover:text-foreground"
        disabled={disabled || sending || !value?.toString().trim()}
        onclick={() => onSend()} title="Send {label}"
    >
        {#if sending}
            <div class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
        {:else}
            <Send class="h-3.5 w-3.5" />
        {/if}
    </Button>
</div>
