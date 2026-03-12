<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { formatHotkey } from "../panel-utils";
    import type { WidgetOverrides } from "../panel-utils";

    interface Props {
        outputId: string;
        xw: any;
        overrides: WidgetOverrides;
        label: string;
        disabled: boolean;
        sending: boolean;
        onSend: (value: string) => void;
    }

    let { outputId, xw, overrides, label, disabled, sending, onSend }: Props =
        $props();
</script>

<Button
    variant={overrides.variant ?? xw.variant ?? "default"}
    class="w-full"
    {disabled}
    onclick={() => onSend(xw.value || "clicked")}
>
    {#if sending}
        <div
            class="h-4 w-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2"
        ></div>
    {/if}
    {label}
    {#if xw.hotkey}
        <kbd
            class="ml-2 pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded border bg-muted/50 px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-70"
        >
            {formatHotkey(xw.hotkey)}
        </kbd>
    {/if}
</Button>
