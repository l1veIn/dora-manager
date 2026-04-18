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

<div class="w-full space-y-2">
    <textarea
        id="widget-{outputId}"
        bind:value
        placeholder={xw.placeholder || `Enter ${label}...`}
        class="w-full min-h-[88px] resize-y rounded-md border border-border/70 bg-background px-3 py-2 text-sm shadow-none transition-shadow focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
        {disabled}
        oninput={() => onValueChange(value)}
        onkeydown={handleKeydown}
    ></textarea>
    <div class="flex items-center justify-between gap-3">
        <p class="text-[11px] text-muted-foreground">
            Press <span class="font-mono">Cmd/Ctrl + Enter</span> or use
            <span class="font-medium">Send</span>.
        </p>
        <Button
            size="sm"
            variant="default"
            class="gap-2"
            disabled={disabled || sending || !value?.toString().trim()}
            onclick={() => onSend()}
            title={`Send ${label}`}
        >
            {#if sending}
                <div class="h-3 w-3 border-2 border-primary-foreground border-t-transparent rounded-full animate-spin"></div>
            {:else}
                <Send class="h-3.5 w-3.5" />
            {/if}
            Send
        </Button>
    </div>
</div>
