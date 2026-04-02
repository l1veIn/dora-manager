<script lang="ts">
    import * as Select from "$lib/components/ui/select/index.js";

    interface Props {
        outputId: string;
        options: any[];
        value: any;
        defaultValue: any;
        disabled?: boolean;
        onValueChange: (v: string) => void;
    }

    let { outputId, options, value, defaultValue, disabled, onValueChange }: Props = $props();

    function optionValue(opt: any) { return typeof opt === "object" ? opt.value : opt; }
    function optionLabel(opt: any) { return typeof opt === "object" ? (opt.label ?? opt.value) : opt; }

    let displayValue = $derived(
        options.find((o) => String(optionValue(o)) === String(value ?? defaultValue))
            ? optionLabel(options.find((o) => String(optionValue(o)) === String(value ?? defaultValue)))
            : (value ?? defaultValue ?? "Select...")
    );
</script>

<Select.Root type="single" value={String(value ?? defaultValue)} onValueChange={(v) => onValueChange(v)}>
    <Select.Trigger class="w-full bg-background shadow-sm" {disabled}>
        {displayValue}
    </Select.Trigger>
    <Select.Content>
        {#each options as opt}
            <Select.Item value={String(optionValue(opt))}>{optionLabel(opt)}</Select.Item>
        {/each}
    </Select.Content>
</Select.Root>
