<script lang="ts">
    import * as Select from "$lib/components/ui/select/index.js";
    import type { NormalizedOption } from "../panel-utils";

    interface Props {
        outputId: string;
        options: NormalizedOption[];
        value: any;
        defaultValue: any;
        disabled: boolean;
        onValueChange: (v: string) => void;
    }

    let { outputId, options, value, defaultValue, disabled, onValueChange }: Props =
        $props();

    let displayValue = $derived(
        options.find((o) => o.value === String(value ?? defaultValue))?.label ??
            value ??
            defaultValue ??
            "Select...",
    );
</script>

<Select.Root
    type="single"
    value={value ?? defaultValue}
    onValueChange={(v) => onValueChange(v)}
>
    <Select.Trigger class="w-full" {disabled}>
        {displayValue}
    </Select.Trigger>
    <Select.Content>
        {#each options as opt}
            <Select.Item value={opt.value}>{opt.label}</Select.Item>
        {/each}
    </Select.Content>
</Select.Root>
