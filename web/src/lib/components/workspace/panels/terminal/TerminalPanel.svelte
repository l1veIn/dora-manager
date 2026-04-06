<script lang="ts">
    import TerminalPane from "../../../../../routes/runs/[id]/TerminalPane.svelte";
    import type { PanelRendererProps } from "../types";

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
</script>

<TerminalPane runId={context.runId} nodeId={selectedNodeId} isRunActive={context.isRunActive} nodes={context.nodes} onNodeChange={handleNodeChange} onClose={() => api.close(item.id)} />
