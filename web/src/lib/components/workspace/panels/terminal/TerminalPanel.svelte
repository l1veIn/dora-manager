<script lang="ts">
    import TerminalPane from "../../../../../routes/runs/[id]/TerminalPane.svelte";
    import type { PanelRendererProps } from "../types";
    import type {
        TerminalThemeOverrides,
        TerminalThemePresetId,
    } from "$lib/terminal/themes";

    let { item, api, context, onConfigChange }: PanelRendererProps = $props();
    let selectedNodeId = $state("");

    $effect(() => {
        if (item.config.nodeId && item.config.nodeId !== selectedNodeId) {
            selectedNodeId = item.config.nodeId;
        }
    });

    function handleNodeChange(newId: string) {
        selectedNodeId = newId;
        if (!item.config) item.config = {};
        item.config.nodeId = newId;
        onConfigChange?.();
    }

    function handleThemePresetChange(nextPreset: TerminalThemePresetId) {
        if (!item.config) item.config = {};
        item.config.themePreset = nextPreset;
        onConfigChange?.();
    }

    function handleThemeOverridesChange(nextOverrides: TerminalThemeOverrides) {
        if (!item.config) item.config = {};
        item.config.themeOverrides = nextOverrides;
        onConfigChange?.();
    }
</script>

<TerminalPane
    runId={context.runId}
    nodeId={selectedNodeId}
    isRunActive={context.isRunActive}
    nodes={context.nodes}
    themePreset={item.config.themePreset}
    themeOverrides={item.config.themeOverrides}
    onNodeChange={handleNodeChange}
    onThemePresetChange={handleThemePresetChange}
    onThemeOverridesChange={handleThemeOverridesChange}
    onClose={() => api.close(item.id)}
/>
