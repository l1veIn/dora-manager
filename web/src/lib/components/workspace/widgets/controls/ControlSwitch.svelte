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

<div class="flex items-center space-x-2 border shadow-sm rounded-lg p-3 bg-muted/20 hover:bg-muted/30 transition-colors">
    <Switch
        id="widget-{outputId}"
        {disabled}
        checked={!!value}
        onCheckedChange={(v) => onValueChange(v)}
    />
    <Label for="widget-{outputId}" class="font-medium cursor-pointer flex-1 text-sm">
        {xw.switchLabel || `Enable`}
    </Label>
</div>
