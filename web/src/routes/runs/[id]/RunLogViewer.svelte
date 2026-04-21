<script lang="ts">
    import { onMount } from "svelte";
    import { getText } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import { createManagedTerminal, type ManagedTerminal } from "$lib/terminal/xterm";
    import { RefreshCw, Download, Dot } from "lucide-svelte";
    import "@xterm/xterm/css/xterm.css";

    let {
        runId = "",
        nodeId = "",
        isRunActive = false,
        nodes = [],
        onNodeChange,
    } = $props<{
        runId: string;
        nodeId: string;
        isRunActive: boolean;
        nodes?: any[];
        onNodeChange?: (id: string) => void;
        onClose?: () => void;
    }>();

    let terminalContainer = $state<HTMLDivElement | null>(null);
    let terminal = $state<ManagedTerminal | null>(null);
    let stream = $state<EventSource | null>(null);
    let resizeObserver = $state<ResizeObserver | null>(null);
    let loading = $state(false);
    let streamState = $state<"idle" | "connecting" | "live" | "closed" | "error">("idle");
    let activeViewKey = $state("");
    let viewKey = $derived(runId && nodeId ? `${runId}:${nodeId}:${isRunActive ? "live" : "done"}` : "");

    function closeStream() {
        if (stream) {
            stream.close();
            stream = null;
        }
    }

    function renderText(text: string) {
        terminal?.resetWithText(text);
        terminal?.fit();
    }

    function appendText(text: string) {
        terminal?.write(text);
    }

    async function fetchFullLog() {
        if (!runId || !nodeId) return "";
        return await getText(`/runs/${runId}/logs/${nodeId}`);
    }

    async function renderFullLog(expectedKey: string) {
        loading = true;
        try {
            const text = await fetchFullLog();
            if (activeViewKey !== expectedKey) return;
            renderText(text);
            streamState = "closed";
        } catch (e) {
            if (activeViewKey !== expectedKey) return;
            renderText("(Failed to load log)");
            streamState = "error";
        } finally {
            if (activeViewKey === expectedKey) {
                loading = false;
            }
        }
    }

    function connectStream(expectedKey: string) {
        if (!runId || !nodeId) return;
        loading = true;
        streamState = "connecting";

        const params = new URLSearchParams({ tail_lines: "800" });
        const source = new EventSource(`/api/runs/${runId}/logs/${nodeId}/stream?${params.toString()}`);

        const ensureCurrent = () => activeViewKey === expectedKey;

        source.onopen = () => {
            if (!ensureCurrent()) return;
            loading = false;
            streamState = "live";
        };

        source.addEventListener("snapshot", (event) => {
            if (!ensureCurrent()) return;
            renderText((event as MessageEvent).data ?? "");
            loading = false;
            streamState = "live";
        });

        source.addEventListener("append", (event) => {
            if (!ensureCurrent()) return;
            appendText((event as MessageEvent).data ?? "");
            streamState = "live";
        });

        source.addEventListener("eof", () => {
            if (!ensureCurrent()) return;
            loading = false;
            streamState = "closed";
            source.close();
            if (stream === source) {
                stream = null;
            }
        });

        source.addEventListener("error", (event) => {
            if (!ensureCurrent()) return;
            const message = (event as MessageEvent).data;
            if (message) {
                renderText(String(message));
            }
            loading = false;
            streamState = "error";
        });

        stream = source;
    }

    async function loadView(nextKey: string) {
        activeViewKey = nextKey;
        closeStream();

        if (!terminal) return;

        if (!runId || !nodeId) {
            loading = false;
            streamState = "idle";
            renderText("");
            return;
        }

        if (isRunActive) {
            renderText("");
            connectStream(nextKey);
        } else {
            await renderFullLog(nextKey);
        }
    }

    async function refreshView() {
        await loadView(viewKey);
    }

    async function downloadFullLog() {
        if (!runId || !nodeId) return;
        const text = await fetchFullLog();
        const blob = new Blob([text], { type: "text/plain" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `${runId}-${nodeId}.log`;
        a.click();
        URL.revokeObjectURL(url);
    }

    let liveHint = $derived(
        streamState === "live"
            ? "Live"
            : streamState === "connecting"
                ? "Connecting"
                : streamState === "error"
                    ? "Stream error"
                    : isRunActive
                        ? "Waiting"
                        : "Static",
    );

    onMount(() => {
        if (terminalContainer) {
            terminal = createManagedTerminal(terminalContainer);
            resizeObserver = new ResizeObserver(() => terminal?.fit());
            resizeObserver.observe(terminalContainer);
        }

        void loadView(viewKey);

        return () => {
            closeStream();
            resizeObserver?.disconnect();
            terminal?.dispose();
        };
    });

    $effect(() => {
        const key = viewKey;
        if (!terminal || key === activeViewKey) return;
        void loadView(key);
    });
</script>

<div class="flex flex-col h-full overflow-hidden w-full bg-background text-foreground">
    <div class="px-4 border-b bg-muted/30 flex items-center justify-between shrink-0 h-11">
        <div class="flex items-center gap-2 min-w-0">
            <select
                class="text-xs font-mono text-muted-foreground bg-muted hover:bg-muted/80 md:px-2 md:py-0.5 rounded px-1.5 py-0 border-0 outline-none ring-0 focus:ring-1 focus:ring-primary cursor-pointer max-w-[140px] truncate"
                value={nodeId}
                onchange={(e) => {
                    const id = e.currentTarget.value;
                    if (onNodeChange) onNodeChange(id);
                }}
            >
                {#if !nodeId}<option value="" disabled hidden>(None Selected)</option>{/if}
                {#each nodes as nItem (nItem.id)}
                    <option value={nItem.id}>{nItem.id}</option>
                {/each}
            </select>
            {#if nodeId}
                <div class="flex items-center gap-1 text-[11px] font-mono text-muted-foreground">
                    <Dot class="size-3.5 {streamState === 'live' ? 'text-emerald-500' : streamState === 'error' ? 'text-red-500' : 'text-muted-foreground'}" />
                    {liveHint}
                </div>
            {/if}
        </div>

        <div class="flex items-center gap-1.5 text-muted-foreground">
            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7 rounded hover:bg-muted hover:text-foreground"
                onclick={refreshView}
                disabled={loading || !nodeId}
                title={isRunActive ? "Reconnect live stream" : "Reload full log"}
            >
                <RefreshCw class="size-3.5 {loading ? 'animate-spin' : ''}" />
            </Button>
            <Button
                variant="ghost"
                size="icon"
                class="h-7 w-7 rounded hover:bg-muted hover:text-foreground"
                onclick={downloadFullLog}
                disabled={!nodeId}
                title="Download full log"
            >
                <Download class="size-3.5" />
            </Button>
        </div>
    </div>

    <div class="flex-1 min-h-0 relative bg-[#0b1020]">
        {#if !nodeId}
            <div class="absolute inset-0 flex flex-col gap-3 items-center justify-center text-slate-400 text-sm font-mono">
                <div>> Select a node from the top left dropdown to view logs</div>
            </div>
        {/if}

        <div class:hidden={!nodeId} class="absolute inset-0 overflow-hidden p-2">
            <div bind:this={terminalContainer} class="h-full w-full rounded-md border border-slate-800 bg-[#0b1020]"></div>
        </div>
    </div>
</div>
