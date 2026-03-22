<script lang="ts">
    import { Copy, Trash2, Plus, MousePointer2, Settings } from 'lucide-svelte';

    let {
        x,
        y,
        type,
        visible,
        onAction,
        onClose,
    }: {
        x: number;
        y: number;
        type: 'pane' | 'node' | 'edge' | null;
        visible: boolean;
        onAction: (action: string) => void;
        onClose: () => void;
    } = $props();

    function handleAction(action: string) {
        onAction(action);
        onClose();
    }
</script>

{#if visible}
    <!-- Full screen transparent backdrop to catch outside clicks -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
        class="fixed inset-0 z-[100]"
        onclick={onClose}
        oncontextmenu={(e) => { e.preventDefault(); onClose(); }}
        role="presentation"
    ></div>

    <div
        class="fixed z-[101] min-w-[160px] bg-popover text-popover-foreground shadow-md border rounded-md p-1 outline-none"
        style="left: {x}px; top: {y}px;"
    >
        {#if type === 'pane'}
            <button class="menu-item" onclick={() => handleAction('addNode')}>
                <Plus class="size-4" />
                <span>Add Node</span>
            </button>
            <button class="menu-item" onclick={() => handleAction('selectAll')}>
                <MousePointer2 class="size-4" />
                <span>Select All</span>
            </button>
        {:else if type === 'node'}
            <button class="menu-item" onclick={() => handleAction('duplicate')}>
                <Copy class="size-4" />
                <span>Duplicate</span>
            </button>
            <button class="menu-item" onclick={() => handleAction('inspect')}>
                <Settings class="size-4" />
                <span>Inspect</span>
            </button>
            <div class="h-px bg-border my-1"></div>
            <button class="menu-item danger" onclick={() => handleAction('deleteNode')}>
                <Trash2 class="size-4" />
                <span>Delete</span>
            </button>
        {:else if type === 'edge'}
            <button class="menu-item danger" onclick={() => handleAction('deleteEdge')}>
                <Trash2 class="size-4" />
                <span>Delete Edge</span>
            </button>
        {/if}
    </div>
{/if}

<style>
    .menu-item {
        display: flex;
        width: 100%;
        align-items: center;
        gap: 8px;
        padding: 6px 8px;
        font-size: 13px;
        border-radius: 4px;
        cursor: pointer;
        background: transparent;
        border: none;
        text-align: left;
        color: inherit;
        transition: background 0.15s, color 0.15s;
    }
    .menu-item:hover {
        background: hsl(var(--accent));
        color: hsl(var(--accent-foreground));
    }
    .menu-item.danger:hover {
        background: hsl(var(--destructive) / 0.1);
        color: hsl(var(--destructive));
    }
</style>
