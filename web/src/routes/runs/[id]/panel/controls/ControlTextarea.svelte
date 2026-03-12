<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { Send } from "lucide-svelte";

    interface Props {
        outputId: string;
        xw: any;
        label: string;
        value: string;
        disabled: boolean;
        sending: boolean;
        onSend: () => void;
        onValueChange: (v: string) => void;
    }

    let {
        outputId,
        xw,
        label,
        value = $bindable(),
        disabled,
        sending,
        onSend,
        onValueChange,
    }: Props = $props();
</script>

<div class="relative group w-full">
    <textarea
        id="widget-{outputId}"
        bind:value
        placeholder={xw.placeholder || `Enter ${label}...`}
        class="w-full min-h-[80px] pl-3 pr-10 py-2 rounded-lg border bg-background text-sm focus-visible:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 transition-shadow resize-y"
        {disabled}
    ></textarea>
    <Button
        size="icon"
        variant="ghost"
        class="absolute right-1 bottom-1 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
        disabled={disabled || sending || !value?.toString().trim()}
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
