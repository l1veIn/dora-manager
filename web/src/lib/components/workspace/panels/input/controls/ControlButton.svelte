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
    class="h-9 w-full rounded-md text-sm shadow-none"
    {disabled}
    onclick={() => onSend(xw.value || "clicked")}
>
    {#if sending}
        <div class="mr-2 h-3.5 w-3.5 rounded-full border-2 border-current border-t-transparent animate-spin"></div>
    {/if}
    {label}
</Button>
