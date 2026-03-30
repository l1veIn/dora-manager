<script lang="ts">
    import { X, Terminal, ArrowRightLeft, Cpu, MemoryStick } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import type { DmFlowNode } from "../../../dataflows/[id]/components/graph/types";
    import { onMount, tick } from "svelte";

    let {
        node,
        logs,
        ioEvents,
        metrics,
        onClose,
    }: {
        node: DmFlowNode;
        logs: string[];
        ioEvents: string[];
        metrics: any;
        onClose: () => void;
    } = $props();

    let logsContainer = $state<HTMLElement | undefined>();
    let ioContainer = $state<HTMLElement | undefined>();
    let autoScroll = $state(true);
    let activeTab = $state<"logs" | "io">("logs");

    function scrollToBottom(container: HTMLElement | undefined) {
        if (container && autoScroll) {
            container.scrollTop = container.scrollHeight;
        }
    }

    $effect(() => {
        if (logs.length || ioEvents.length) {
            tick().then(() => {
                if (activeTab === "logs") scrollToBottom(logsContainer);
                else scrollToBottom(ioContainer);
            });
        }
    });

    function handleScroll(e: Event) {
        const target = e.target as HTMLElement;
        const isAtBottom = target.scrollHeight - target.scrollTop <= target.clientHeight + 10;
        autoScroll = isAtBottom;
    }
</script>

<div class="absolute right-0 top-0 bottom-0 w-80 bg-background/95 backdrop-blur-md border-l shadow-2xl z-50 flex flex-col font-mono text-sm">
    <div class="px-4 py-3 border-b flex items-center justify-between shrink-0 bg-muted/20">
        <h3 class="font-bold flex items-center gap-2">
            {node.data.label}
            <span class="text-[10px] font-normal uppercase text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
                {(node.data.nodeType as string).split('/').pop()}
            </span>
        </h3>
        <Button variant="ghost" size="icon" class="h-6 w-6" onclick={onClose}>
            <X class="size-4" />
        </Button>
    </div>

    <!-- Metrics Summary -->
    {#if metrics}
    <div class="px-4 py-2 border-b bg-muted/10 flex gap-4 text-xs font-semibold shrink-0">
        <div class="flex items-center text-blue-500 gap-1.5" title="CPU">
            <Cpu class="size-3.5" /> {metrics.cpu?.toFixed(2) || '0.00'}%
        </div>
        <div class="flex items-center text-purple-500 gap-1.5" title="Memory">
            <MemoryStick class="size-3.5" /> {metrics.memory ? (metrics.memory / 1024 / 1024).toFixed(1) : '0'} MB
        </div>
    </div>
    {/if}

    <!-- Tabs -->
    <div class="flex border-b shrink-0 bg-card">
        <button 
            class="flex-1 py-2 text-xs font-semibold border-b-2 flex items-center justify-center gap-1.5 transition-colors"
            class:border-foreground={activeTab === 'logs'}
            class:text-foreground={activeTab === 'logs'}
            class:border-transparent={activeTab !== 'logs'}
            class:text-muted-foreground={activeTab !== 'logs'}
            onclick={() => activeTab = 'logs'}
        >
            <Terminal class="size-3.5" /> Stdout Logs
        </button>
        <button 
            class="flex-1 py-2 text-xs font-semibold border-b-2 flex items-center justify-center gap-1.5 transition-colors"
            class:border-foreground={activeTab === 'io'}
            class:text-foreground={activeTab === 'io'}
            class:border-transparent={activeTab !== 'io'}
            class:text-muted-foreground={activeTab !== 'io'}
            onclick={() => activeTab = 'io'}
        >
            <ArrowRightLeft class="size-3.5" /> [DM-IO] Flow
        </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-hidden relative">
        {#if activeTab === "logs"}
            <div 
                class="absolute inset-0 overflow-y-auto p-3 text-[11px] leading-relaxed break-all" 
                bind:this={logsContainer}
                onscroll={handleScroll}
            >
                {#each logs as line}
                    <div class="border-b border-border/40 py-1 font-mono break-words whitespace-pre-wrap">{line}</div>
                {/each}
                {#if logs.length === 0}
                    <div class="h-full flex items-center justify-center text-muted-foreground/50">No logs captured</div>
                {/if}
            </div>
        {:else}
            <div 
                class="absolute inset-0 overflow-y-auto p-3 text-[11px] leading-relaxed break-all" 
                bind:this={ioContainer}
                onscroll={handleScroll}
            >
                {#each ioEvents as line}
                    <div class="py-1 text-sky-500 border-b border-sky-500/10 mb-1">{line}</div>
                {/each}
                {#if ioEvents.length === 0}
                    <div class="h-full flex flex-col gap-2 items-center justify-center text-muted-foreground/50">
                        <ArrowRightLeft class="size-8" />
                        <span>No I/O events parsed yet</span>
                    </div>
                {/if}
            </div>
        {/if}
    </div>
    
    <!-- Autoscroll Indicator -->
    {#if !autoScroll && ((activeTab === 'logs' && logs.length > 0) || (activeTab === 'io' && ioEvents.length > 0))}
        <div class="absolute bottom-4 left-0 right-0 flex justify-center shrink-0">
            <button 
                class="bg-foreground text-background text-[10px] px-3 py-1 rounded-full shadow-lg font-semibold"
                onclick={() => { autoScroll = true; scrollToBottom(activeTab === 'logs' ? logsContainer : ioContainer); }}
            >
                Resume Auto-scroll
            </button>
        </div>
    {/if}
</div>
