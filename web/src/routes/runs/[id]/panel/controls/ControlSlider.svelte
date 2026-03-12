<script lang="ts">
    import { Slider } from "$lib/components/ui/slider/index.js";

    interface Props {
        outputId: string;
        xw: any;
        value: any;
        defaultValue: any;
        disabled: boolean;
        overrideValue?: any;
        onValueChange: (v: any) => void;
    }

    let { outputId, xw, value, defaultValue, disabled, overrideValue, onValueChange }: Props =
        $props();

    let current = $derived(overrideValue ?? value ?? defaultValue ?? 0);
</script>

<div class="flex items-center gap-4">
    <Slider
        type="single"
        value={current}
        min={xw.min ?? 0}
        max={xw.max ?? 100}
        step={xw.step ?? 1}
        {disabled}
        onValueChange={(v) => onValueChange(v)}
        class="flex-1"
    />
    <span
        class="w-14 text-center text-sm font-mono tabular-nums text-muted-foreground"
    >
        {current}
    </span>
</div>
