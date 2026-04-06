<script lang="ts">
    import { Slider } from "$lib/components/ui/slider/index.js";

    interface Props {
        outputId: string;
        xw: any;
        value: number;
        defaultValue: number;
        disabled?: boolean;
        onValueChange: (v: number) => void;
    }

    let { outputId, xw, value, defaultValue, disabled, onValueChange }: Props = $props();
</script>

<div class="flex items-center gap-3 rounded-md border border-border/70 bg-muted/10 px-3 py-2.5">
    <Slider
        id="widget-{outputId}"
        {disabled}
        value={(value !== undefined ? value : defaultValue) ?? 0}
        type="single"
        min={xw.min ?? 0}
        max={xw.max ?? 100}
        step={xw.step ?? 1}
        onValueChange={(v: number) => onValueChange(v)}
        class="flex-1"
    />
    <span class="w-8 text-right font-mono text-[11px] text-muted-foreground">
        {(value !== undefined ? value : defaultValue) ?? 0}
    </span>
</div>
