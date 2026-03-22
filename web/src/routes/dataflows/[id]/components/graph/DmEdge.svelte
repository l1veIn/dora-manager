<script lang="ts">
    import { BaseEdge, getBezierPath, useSvelteFlow } from '@xyflow/svelte';
    import { X } from 'lucide-svelte';

    let {
        id,
        sourceX,
        sourceY,
        targetX,
        targetY,
        sourcePosition,
        targetPosition,
        selected = false,
        markerEnd
    }: any = $props();

    const { deleteElements } = useSvelteFlow();

    let pathParams = $derived(
        getBezierPath({
            sourceX,
            sourceY,
            sourcePosition,
            targetX,
            targetY,
            targetPosition,
        })
    );
    let edgePath = $derived(pathParams[0]);
    let labelX = $derived(pathParams[1]);
    let labelY = $derived(pathParams[2]);

    function handleDelete(e: MouseEvent) {
        e.stopPropagation();
        deleteElements({ edges: [{ id }] });
    }
</script>

<BaseEdge path={edgePath} {id} {markerEnd} interactionWidth={20} />

<foreignObject
    width={40}
    height={40}
    x={labelX - 20}
    y={labelY - 20}
    class="edge-foreign-object nodrag nopan"
    requiredExtensions="http://www.w3.org/1999/xhtml"
>
    <div class="edge-btn-container">
        <button
            class="edge-delete-btn"
            class:selected
            onclick={handleDelete}
            title="Delete Edge"
        >
            <X class="w-3 h-3" />
        </button>
    </div>
</foreignObject>

<style>
    .edge-foreign-object {
        overflow: visible;
        pointer-events: none;
    }
    
    .edge-btn-container {
        pointer-events: all;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
    }

    .edge-delete-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 20px;
        height: 20px;
        border-radius: 50%;
        background: hsl(var(--card));
        border: 1px solid hsl(var(--border));
        color: hsl(var(--muted-foreground));
        cursor: pointer;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        opacity: 0;
        transition: all 0.2s;
    }

    .edge-delete-btn:hover {
        background: hsl(var(--destructive));
        color: hsl(var(--destructive-foreground));
        border-color: hsl(var(--destructive));
        opacity: 1 !important;
        transform: scale(1.1);
    }

    .edge-delete-btn.selected {
        opacity: 1;
        border-color: hsl(var(--primary));
        color: hsl(var(--primary));
    }
    
    /* Make button appear when hovering the edge container */
    :global(.svelte-flow__edge:hover) .edge-delete-btn {
        opacity: 1;
    }
</style>
