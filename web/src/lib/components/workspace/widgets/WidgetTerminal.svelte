<script lang="ts">
    import TerminalPane from "../../../../routes/runs/[id]/TerminalPane.svelte";
    
    let { node, api, runId, nodes = [], onConfigChange } = $props<{ node: any; api: any; runId: string; nodes: any[]; onConfigChange?: () => void }>();

    let selectedNodeId = $state(node.config.nodeId || "");

    $effect(() => {
        if (node.config.nodeId && node.config.nodeId !== selectedNodeId) {
            selectedNodeId = node.config.nodeId;
        }
    });

    function handleNodeChange(newId: string) {
        selectedNodeId = newId;
        if (!node.config) node.config = {};
        node.config.nodeId = newId;
        if (onConfigChange) onConfigChange();
    }
</script>

<TerminalPane {runId} nodeId={selectedNodeId} isRunActive={true} {nodes} onNodeChange={handleNodeChange} onClose={() => api.close(node.id)} />
