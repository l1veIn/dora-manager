<script lang="ts">
    import RunLogViewer from "./RunLogViewer.svelte";
    import type {
        TerminalThemeOverrides,
        TerminalThemePresetId,
    } from "$lib/terminal/themes";

    interface Props {
        runId: string;
        nodeId: string;
        isRunActive: boolean;
        nodes?: any[];
        themePreset?: TerminalThemePresetId;
        themeOverrides?: TerminalThemeOverrides;
        onNodeChange?: (id: string) => void;
        onThemePresetChange?: (preset: TerminalThemePresetId) => void;
        onThemeOverridesChange?: (overrides: TerminalThemeOverrides) => void;
        onClose: () => void;
    }

    let {
        runId,
        nodeId,
        isRunActive,
        nodes = [],
        themePreset = "auto",
        themeOverrides = {},
        onNodeChange,
        onThemePresetChange,
        onThemeOverridesChange,
        onClose,
    }: Props = $props();
</script>

<RunLogViewer
    {runId}
    {nodeId}
    {isRunActive}
    {nodes}
    {themePreset}
    {themeOverrides}
    {onNodeChange}
    {onThemePresetChange}
    {onThemeOverridesChange}
    {onClose}
/>
