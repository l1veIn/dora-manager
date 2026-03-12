<script lang="ts">
    import { PathPicker } from "$lib/components/ui/path-picker/index.js";

    interface Props {
        outputId: string;
        xw: any;
        mode: "file" | "directory";
        value: string;
        disabled: boolean;
        sending: boolean;
        onSend: () => void;
        onValueChange: (v: string) => void;
    }

    let {
        outputId,
        xw,
        mode,
        value = $bindable(),
        disabled,
        sending,
        onSend,
        onValueChange,
    }: Props = $props();
</script>

<PathPicker
    {mode}
    id="widget-{outputId}"
    bind:value
    placeholder={xw.placeholder || undefined}
    {disabled}
    showConfirmBtn={true}
    confirming={sending}
    onConfirm={() => onSend()}
/>
