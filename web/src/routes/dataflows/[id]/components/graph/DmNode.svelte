<script lang="ts">
    import { Handle, Position } from '@xyflow/svelte';

    let { data, id }: { data: Record<string, any>; id: string } = $props();

    // Use $derived to track reactive data fields
    let label = $derived(data.label as string);
    let nodeType = $derived(data.nodeType as string);
    let inputs = $derived((data.inputs || []) as string[]);
    let outputs = $derived((data.outputs || []) as string[]);
    let isVirtual = $derived(data.isVirtual as boolean | undefined);
    let virtualKind = $derived(data.virtualKind as 'timer' | 'panel' | undefined);
</script>

<div
    class="dm-node"
    class:dm-node--virtual={isVirtual}
    class:dm-node--timer={virtualKind === 'timer'}
    class:dm-node--panel={virtualKind === 'panel'}
>
    <div class="dm-node__header">
        <span class="dm-node__label">{label}</span>
        {#if !isVirtual && nodeType !== label}
            <span class="dm-node__type">{nodeType}</span>
        {/if}
    </div>

    <div class="dm-node__body">
        <div class="dm-node__col">
            {#each inputs as port, i}
                <div class="dm-node__port">
                    <Handle
                        type="target"
                        position={Position.Left}
                        id={`in-${port}`}
                    />
                    <span class="dm-node__port-label">← {port}</span>
                </div>
            {/each}
        </div>

        <div class="dm-node__col dm-node__col--right">
            {#each outputs as port, i}
                <div class="dm-node__port dm-node__port--out">
                    <span class="dm-node__port-label">{port} →</span>
                    <Handle
                        type="source"
                        position={Position.Right}
                        id={`out-${port}`}
                    />
                </div>
            {/each}
        </div>
    </div>
</div>

<style>
    /* ── Light mode (default) ── */
    .dm-node {
        background: #ffffff;
        border: 1.5px solid #e2e8f0;
        border-radius: 10px;
        min-width: 180px;
        max-width: 280px;
        font-size: 13px;
        box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08), 0 2px 8px rgba(0, 0, 0, 0.04);
        transition: box-shadow 0.15s ease;
        color: #1a202c;
    }
    .dm-node:hover {
        box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    }

    /* ── Dark mode ── */
    :global(.dark) .dm-node {
        background: #1e293b;
        border-color: #334155;
        color: #e2e8f0;
        box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3), 0 2px 8px rgba(0, 0, 0, 0.2);
    }
    :global(.dark) .dm-node:hover {
        box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    }

    /* ── Virtual nodes ── */
    .dm-node--virtual {
        border-style: dashed;
        opacity: 0.9;
    }
    .dm-node--timer {
        border-color: #3b82f6;
    }
    .dm-node--timer .dm-node__header {
        background: rgba(59, 130, 246, 0.08);
    }
    :global(.dark) .dm-node--timer .dm-node__header {
        background: rgba(59, 130, 246, 0.15);
    }

    .dm-node--panel {
        border-color: #8b5cf6;
    }
    .dm-node--panel .dm-node__header {
        background: rgba(139, 92, 246, 0.08);
    }
    :global(.dark) .dm-node--panel .dm-node__header {
        background: rgba(139, 92, 246, 0.15);
    }

    /* ── Header ── */
    .dm-node__header {
        position: relative;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 7px 12px 7px 14px;
        border-bottom: 1px solid #e2e8f0;
        background: #f8fafc;
        border-radius: 10px 10px 0 0;
        overflow: hidden;
    }
    .dm-node__header::before {
        content: '';
        position: absolute;
        left: 0;
        top: 0;
        bottom: 0;
        width: 4px;
        background: #94a3b8;
    }
    .dm-node--timer .dm-node__header::before { background: #3b82f6; }
    .dm-node--panel .dm-node__header::before { background: #8b5cf6; }

    :global(.dark) .dm-node__header {
        border-bottom-color: #334155;
        background: #0f172a;
    }

    .dm-node__label {
        font-weight: 600;
        font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
        font-size: 12px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .dm-node__type {
        font-size: 10px;
        color: #94a3b8;
        margin-left: auto;
        white-space: nowrap;
    }

    /* ── Body ── */
    .dm-node__body {
        display: flex;
        justify-content: space-between;
        gap: 12px;
        padding: 8px 0;
        min-height: 28px;
    }
    .dm-node__col {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
    .dm-node__col--right {
        align-items: flex-end;
    }
    .dm-node__port {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 2px 12px;
        position: relative;
        height: 24px;
        transition: background 0.15s ease;
    }
    .dm-node__port:hover {
        background: rgba(0, 0, 0, 0.04);
    }
    :global(.dark) .dm-node__port:hover {
        background: rgba(255, 255, 255, 0.05);
    }
    .dm-node__port--out {
        flex-direction: row;
    }
    .dm-node__port-label {
        font-size: 11px;
        color: #64748b;
        font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
        white-space: nowrap;
    }
    :global(.dark) .dm-node__port-label {
        color: #94a3b8;
    }
</style>
