<script lang="ts">
    import RunLogViewer from "./RunLogViewer.svelte";

    interface Props {
        runId: string;
        nodeId: string;
        isRunActive: boolean;
        onClose: () => void;
    }

    let { runId, nodeId, isRunActive, onClose }: Props = $props();
</script>

<RunLogViewer {runId} {nodeId} {isRunActive} {onClose} />
