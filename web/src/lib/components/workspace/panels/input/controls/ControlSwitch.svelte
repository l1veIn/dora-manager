<script lang="ts">
    import { Switch } from "$lib/components/ui/switch/index.js";
    import { Label } from "$lib/components/ui/label/index.js";

    interface Props {
        outputId: string;
        xw: any;
        value: any;
        disabled?: boolean;
        onValueChange: (v: boolean) => void;
    }

    let { outputId, xw, value, disabled, onValueChange }: Props = $props();
</script>

<div class="flex items-center gap-2 rounded-md border border-border/70 bg-muted/10 px-3 py-2">
    <Switch
        id="widget-{outputId}"
        {disabled}
        checked={!!value}
        onCheckedChange={(v) => onValueChange(v)}
    />
    <Label for="widget-{outputId}" class="flex-1 cursor-pointer text-sm font-medium">
        {xw.switchLabel || `Enable`}
    </Label>
</div>
