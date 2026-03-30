<script lang="ts">
    import { Handle, Position } from '@xyflow/svelte';
    import { Cpu, MemoryStick, Activity, CheckCircle2, XCircle, AlertCircle, PlayCircle } from 'lucide-svelte';

    let { data, id }: { data: Record<string, any>; id: string } = $props();

    // Base data
    let label = $derived(data.label as string);
    let nodeType = $derived(data.nodeType as string);
    let inputs = $derived((data.inputs || []) as string[]);
    let outputs = $derived((data.outputs || []) as string[]);
    let isVirtual = $derived(data.isVirtual as boolean | undefined);
    let virtualKind = $derived(data.virtualKind as 'timer' | 'panel' | undefined);

    // Runtime data injected via ws push
    let status = $derived(data.status as string | undefined || 'unknown'); // 'running', 'stopped', 'failed', 'unknown'
    let cpu = $derived(data.cpu as number | undefined);
    let memory = $derived(data.memory as number | undefined); // bytes
    let hasLogs = $derived(data.hasLogs as boolean | undefined);
    
    function formatMemory(bytes?: number) {
        if (!bytes) return '0 MB';
        return (bytes / 1024 / 1024).toFixed(1) + ' MB';
    }
</script>

<div
    class="dm-node"
    class:dm-node--virtual={isVirtual}
    class:dm-node--timer={virtualKind === 'timer'}
    class:dm-node--panel={virtualKind === 'panel'}
    class:status-running={status === 'running'}
    class:status-failed={status === 'failed'}
    class:status-stopped={status === 'stopped'}
>
    <!-- Node Header -->
    <div class="dm-node__header">
        <span class="dm-node__label">{label}</span>
        
        <!-- Status indicator icon -->
        <span class="dm-node__status-icon">
            {#if status === 'running'}
                <PlayCircle class="size-4 text-green-500 animate-pulse" />
            {:else if status === 'failed'}
                <XCircle class="size-4 text-red-500" />
            {:else if status === 'stopped'}
                <CheckCircle2 class="size-4 text-slate-500" />
            {:else}
                <Activity class="size-4 text-muted-foreground" />
            {/if}
        </span>
    </div>

    <!-- Configuration info -->
    {#if !isVirtual && nodeType !== label}
        <div class="dm-node__type-banner">
            {nodeType}
        </div>
    {/if}

    <div class="dm-node__body">
        <div class="dm-node__col">
            {#each inputs as port, i}
                <div class="dm-node__port">
                    <Handle
                        type="target"
                        position={Position.Left}
                        id={`in-${port}`}
                        class="runtime-handle"
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
                        class="runtime-handle"
                    />
                </div>
            {/each}
        </div>
    </div>
    
    <!-- Runtime Metrics Footer -->
    {#if cpu !== undefined || memory !== undefined || hasLogs}
        <div class="dm-node__footer">
            <div class="metrics">
                {#if cpu !== undefined}
                    <span class="metric" title="CPU Usage">
                        <Cpu class="size-3 mr-1 text-blue-500" /> {cpu.toFixed(1)}%
                    </span>
                {/if}
                {#if memory !== undefined}
                    <span class="metric" title="Memory Usage">
                        <MemoryStick class="size-3 mr-1 text-purple-500" /> {formatMemory(memory)}
                    </span>
                {/if}
            </div>
            {#if hasLogs}
            <div class="logs-indicator">
                <span class="relative flex h-2 w-2">
                  <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-sky-400 opacity-75"></span>
                  <span class="relative inline-flex rounded-full h-2 w-2 bg-sky-500"></span>
                </span>
            </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    /* ── Base ── */
    .dm-node {
        background: #ffffff;
        border: 2px solid #e2e8f0;
        border-radius: 10px;
        min-width: 200px;
        max-width: 300px;
        font-size: 13px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
        color: #1a202c;
        overflow: hidden;
        transition: all 0.2s ease;
    }

    /* ── Dark mode ── */
    :global(.dark) .dm-node {
        background: #1e293b;
        border-color: #334155;
        color: #e2e8f0;
    }

    /* ── Status Borders ── */
    .status-running { border-color: #22c55e; box-shadow: 0 0 0 2px rgba(34, 197, 94, 0.2); }
    .status-failed { border-color: #ef4444; }
    .status-stopped { border-color: #64748b; opacity: 0.8; }
    
    :global(.dark) .status-running { border-color: #22c55e; box-shadow: 0 0 0 2px rgba(34, 197, 94, 0.1); }
    :global(.dark) .status-failed { border-color: #ef4444; }

    /* ── Header ── */
    .dm-node__header {
        position: relative;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 12px;
        background: #f8fafc;
        border-bottom: 1px solid #e2e8f0;
    }
    :global(.dark) .dm-node__header {
        background: #0f172a;
        border-bottom-color: #334155;
    }

    .dm-node__label {
        font-weight: 600;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        font-size: 13px;
    }
    
    .dm-node__type-banner {
        font-size: 10px;
        color: #ffffff;
        background: #64748b;
        padding: 2px 12px;
        font-weight: 500;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    :global(.dark) .dm-node__type-banner {
        background: #475569;
    }

    /* ── Body ── */
    .dm-node__body {
        display: flex;
        justify-content: space-between;
        gap: 12px;
        padding: 10px 0;
    }
    .dm-node__col { display: flex; flex-direction: column; gap: 4px; }
    .dm-node__col--right { align-items: flex-end; }
    
    .dm-node__port {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 2px 12px;
        position: relative;
        height: 24px;
    }
    .dm-node__port-label {
        font-size: 11px;
        color: #64748b;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    }
    :global(.dark) .dm-node__port-label { color: #94a3b8; }

    /* ── Custom Handles ── */
    :global(.runtime-handle) {
        width: 10px !important;
        height: 10px !important;
        background: #fff !important;
        border: 2px solid #94a3b8 !important;
    }
    .status-running :global(.runtime-handle) { border-color: #22c55e !important; }
    .status-failed :global(.runtime-handle) { border-color: #ef4444 !important; }
    
    /* ── Footer / Metrics ── */
    .dm-node__footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 6px 10px;
        background: #f1f5f9;
        border-top: 1px solid #e2e8f0;
        font-size: 11px;
        color: #475569;
    }
    :global(.dark) .dm-node__footer {
        background: #0b1120;
        border-top-color: #334155;
        color: #94a3b8;
    }
    .metrics {
        display: flex;
        gap: 10px;
    }
    .metric {
        display: flex;
        align-items: center;
        font-family: ui-monospace, Menlo, monospace;
    }
    .logs-indicator {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2px;
    }
</style>
