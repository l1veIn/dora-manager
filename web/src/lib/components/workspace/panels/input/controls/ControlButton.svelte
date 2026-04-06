<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";

    interface Props {
        outputId: string;
        xw: any;
        label: string;
        disabled?: boolean;
        sending?: boolean;
        onSend: (value: string) => void;
    }

    let { outputId, xw, label, disabled = false, sending = false, onSend }: Props = $props();
</script>

<Button
    variant={xw.variant ?? "default"}
    class="w-full"
    {disabled}
    onclick={() => onSend(xw.value || "clicked")}
>
    {#if sending}
        <div class="h-4 w-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2"></div>
    {/if}
    {label}
</Button>
