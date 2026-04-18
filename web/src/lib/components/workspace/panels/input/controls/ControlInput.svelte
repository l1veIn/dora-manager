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

<div class="w-full space-y-2">
    <div class="flex items-center gap-2">
        <Input
            id="widget-{outputId}"
            bind:value
            placeholder={xw.placeholder || `Enter ${label}...`}
            class="h-10 flex-1 rounded-md border-border/70 bg-background text-sm shadow-none"
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
            size="sm"
            variant="default"
            class="gap-2 shrink-0"
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
    <p class="text-[11px] text-muted-foreground">
        Press <span class="font-mono">Enter</span> or use
        <span class="font-medium">Send</span>.
    </p>
</div>
