<script lang="ts">
    import { getContext } from "svelte";
    import { getText } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import { RefreshCw, Download } from "lucide-svelte";

    let {
        runId = "",
        nodeId = "",
        isRunActive = false,
        nodes = [],
        onNodeChange,
        onClose,
    } = $props<{
        runId: string;
        nodeId: string;
        isRunActive: boolean;
        nodes?: any[];
        onNodeChange?: (id: string) => void;
        onClose?: () => void;
    }>();

    // Direct access to shared $state proxy — bypasses prop chain entirely
    const logsMap = getContext<Record<string, string>>('runLogs');

    let fetchedLog = $state<string>("");
    let loading = $state(false);
    let activeLogKey = $state("");

    let logContainer = $state<HTMLElement | null>(null);
    let autoScroll = $state(true);

    // Active run: read directly from shared $state proxy. Completed run: HTTP fetch.
    let logContent = $derived(isRunActive ? (logsMap?.[nodeId] ?? "") : fetchedLog);

    async function fetchFullLog() {
        if (!runId || !nodeId) return;
        loading = true;
        try {
            const text = await getText(`/runs/${runId}/logs/${nodeId}`);
            fetchedLog = text;
        } catch (e) {
            fetchedLog = "(Failed to load log)";
        } finally {
            loading = false;
            scrollToBottom();
        }
    }

    function scrollToBottom() {
        if (logContainer && autoScroll) {
            setTimeout(() => {
                if (logContainer) {
                    logContainer.scrollTop = logContainer.scrollHeight;
                }
            }, 10);
        }
    }

    function handleScroll() {
        if (!logContainer) return;
        const { scrollTop, scrollHeight, clientHeight } = logContainer;
        autoScroll =
            Math.abs(scrollHeight - clientHeight - scrollTop) < 10;
    }

    function escapeHtml(unsafe: string) {
        return unsafe
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }

    let formattedLog = $derived.by(() => {
        if (!logContent) return "";
        return logContent
            .split('\n')
            .map((line: string) => {
                const escaped = escapeHtml(line);
                if (escaped.includes("[DM-IO]")) {
                    return `<span class="text-sky-500">${escaped}</span>`;
                }
                return escaped;
            })
            .join('\n');
    });

    // Auto-scroll when new logs arrive
    $effect(() => {
        const _ = logContent;
        if (autoScroll && logContainer) scrollToBottom();
    });

    // Fetch full log for completed runs (initial + node switch)
    $effect(() => {
        const key = runId && nodeId ? `${runId}:${nodeId}` : "";
        if (key === activeLogKey) return;
        activeLogKey = key;
        fetchedLog = "";

        if (key && !isRunActive) {
            fetchFullLog();
        }
    });
</script>

<div
    class="flex flex-col h-full overflow-hidden w-full bg-background text-foreground"
>
    <div
        class="px-4 border-b bg-muted/30 flex items-center justify-between shrink-0 h-11"
    >
        <div class="flex items-center gap-2">
            <select
                class="text-xs font-mono text-muted-foreground bg-muted hover:bg-muted/80 md:px-2 md:py-0.5 rounded px-1.5 py-0 border-0 outline-none ring-0 focus:ring-1 focus:ring-primary cursor-pointer max-w-[140px] truncate"
                value={nodeId}
                onchange={(e) => {
                    const id = e.currentTarget.value;
                    if (onNodeChange) onNodeChange(id);
                }}
            >
                {#if !nodeId}<option value="" disabled hidden
                        >(None Selected)</option
                    >{/if}
                {#each nodes as nItem (nItem.id)}
                    <option value={nItem.id}>{nItem.id}</option>
                {/each}
            </select>
        </div>

        <div class="flex items-center gap-1.5 text-muted-foreground">
            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7 rounded hover:bg-muted hover:text-foreground"
                onclick={fetchFullLog}
                disabled={loading || !nodeId}
                title="Refresh full log"
            >
                <RefreshCw class="size-3.5 {loading ? 'animate-spin' : ''}" />
            </Button>
            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7 rounded hover:bg-muted hover:text-foreground"
                onclick={() => {
                    const blob = new Blob([logContent], { type: "text/plain" });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = `${runId}-${nodeId}.log`;
                    a.click();
                }}
                disabled={!logContent}
                title="Download"
            >
                <Download class="size-3.5" />
            </Button>
        </div>
    </div>

    <div
        class="p-0 flex-1 overflow-auto h-full relative"
        bind:this={logContainer}
        onscroll={handleScroll}
    >
        {#if !nodeId}
            <div
                class="absolute inset-0 flex flex-col gap-3 items-center justify-center text-muted-foreground text-sm font-mono"
            >
                <div>
                    > Select a node from the top left dropdown to view logs
                </div>
            </div>
        {:else if loading && !logContent}
            <div
                class="absolute inset-0 flex items-center justify-center text-muted-foreground text-sm font-mono"
            >
                > Loading trace...
            </div>
        {:else}
            <div
                class="p-4 font-mono text-[13px] whitespace-pre-wrap break-all leading-relaxed text-foreground selection:bg-muted"
            >
                {#if logContent === ""}
                    (NO LOG OUTPUT)
                {:else}
                    {@html formattedLog}
                {/if}
            </div>
        {/if}
    </div>
</div>
