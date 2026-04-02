<script lang="ts">
    import RunLogViewer from "./RunLogViewer.svelte";

    interface Props {
        runId: string;
        nodeId: string;
        isRunActive: boolean;
        nodes?: any[];
        onNodeChange?: (id: string) => void;
        onClose: () => void;
    }

    let { runId, nodeId, isRunActive, nodes = [], onNodeChange, onClose }: Props = $props();
</script>

<RunLogViewer {runId} {nodeId} {isRunActive} {nodes} {onNodeChange} {onClose} />
