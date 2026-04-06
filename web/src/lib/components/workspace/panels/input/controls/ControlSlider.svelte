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

<div class="flex items-center space-x-4 border shadow-sm rounded-lg p-3 bg-muted/20">
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
    <span class="text-xs font-mono w-10 text-right text-muted-foreground border rounded bg-background px-1 py-0.5 shadow-sm">
        {(value !== undefined ? value : defaultValue) ?? 0}
    </span>
</div>
