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
        class="h-10 w-full rounded-md border-border/70 bg-background pr-9 text-sm shadow-none"
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
        size="icon" variant="ghost" class="absolute right-0.5 top-0.5 h-9 w-9 text-muted-foreground hover:text-foreground"
        disabled={disabled || sending || !value?.toString().trim()}
        onclick={() => onSend()} title="Send {label}"
    >
        {#if sending}
            <div class="h-3 w-3 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
        {:else}
            <Send class="h-3.5 w-3.5" />
        {/if}
    </Button>
</div>
