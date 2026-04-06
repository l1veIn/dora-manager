<script lang="ts">
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

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && value) {
            e.preventDefault();
            onSend();
        }
    }
</script>

<div class="relative group w-full">
    <textarea
        id="widget-{outputId}"
        bind:value
        placeholder={xw.placeholder || `Enter ${label}...`}
        class="w-full min-h-[68px] resize-y rounded-md border border-border/70 bg-background px-3 py-2 pr-9 text-sm shadow-none transition-shadow focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
        {disabled}
        oninput={() => onValueChange(value)}
        onkeydown={handleKeydown}
    ></textarea>
    <Button
        size="icon" variant="ghost" class="absolute bottom-0.5 right-0.5 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
        disabled={disabled || sending || !value?.toString().trim()}
        onclick={() => onSend()} title="Send {label} (Cmd+Enter)"
    >
        {#if sending}
            <div class="h-3 w-3 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
        {:else}
            <Send class="h-3.5 w-3.5" />
        {/if}
    </Button>
</div>
