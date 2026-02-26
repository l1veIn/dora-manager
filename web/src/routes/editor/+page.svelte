<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { post, get } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import * as Resizable from "$lib/components/ui/resizable/index.js";
    import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
    import { Play, Square, Code, ActivitySquare } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import CodeMirror from "svelte-codemirror-editor";
    import { yaml } from "@codemirror/lang-yaml";

    let code = $state(`nodes:
  - id: custom_node
    operator:
      python: |
        def process(event, state):
            return event
`);
    let isRunning = $state(false);
    let outputLogs = $state<string[]>([]);
    let monitorInterval: ReturnType<typeof setInterval>;

    async function runDataflow() {
        if (isRunning) return;
        isRunning = true;
        outputLogs = [
            `[${new Date().toLocaleTimeString()}] Starting dataflow...`,
        ];
        try {
            const res: any = await post("/dataflow/run", { yaml: code });
            outputLogs = [
                ...outputLogs,
                `[${new Date().toLocaleTimeString()}] ${res.message}`,
            ];
            startMonitoring();
        } catch (e: any) {
            toast.error(`Run failed: ${e.message}`);
            outputLogs = [
                ...outputLogs,
                `[${new Date().toLocaleTimeString()}] ERROR: ${e.message}`,
            ];
            isRunning = false;
        }
    }

    async function stopDataflow() {
        if (!isRunning) return;
        try {
            const res: any = await post("/dataflow/stop");
            outputLogs = [
                ...outputLogs,
                `[${new Date().toLocaleTimeString()}] ${res.message}`,
            ];
        } catch (e: any) {
            toast.error(`Stop failed: ${e.message}`);
        } finally {
            isRunning = false;
            stopMonitoring();
        }
    }

    function startMonitoring() {
        stopMonitoring();
        monitorInterval = setInterval(async () => {
            try {
                const events: any[] = await get("/events?source=core&limit=10");
                // Just appending any new events to the log for basic demo
                events.forEach((ev) => {
                    const logStr = `[${new Date(ev.timestamp).toLocaleTimeString()}] [${ev.source}] ${ev.activity}`;
                    if (!outputLogs.includes(logStr)) {
                        outputLogs = [...outputLogs, logStr].slice(-100);
                    }
                });
            } catch (e) {
                // ignore polling errors
            }
        }, 2000);
    }

    function stopMonitoring() {
        if (monitorInterval) clearInterval(monitorInterval);
    }

    onDestroy(() => {
        stopMonitoring();
    });
</script>

<div class="h-full flex flex-col pt-4">
    <div class="flex items-center justify-between px-6 pb-4">
        <div class="flex items-center gap-2">
            <div class="p-2 bg-primary/10 rounded-md">
                <Code class="size-5 text-primary" />
            </div>
            <div>
                <h1 class="text-2xl font-bold tracking-tight">Editor</h1>
                <p class="text-sm text-muted-foreground hidden sm:block">
                    Design and execute Dora dataflows.
                </p>
            </div>
        </div>

        <div class="flex items-center gap-2">
            <Button
                variant="default"
                size="sm"
                disabled={isRunning}
                onclick={runDataflow}
            >
                <Play class="mr-2 size-4" />
                Run
            </Button>
            <Button
                variant="destructive"
                size="sm"
                disabled={!isRunning}
                onclick={stopDataflow}
                class="min-w-24"
            >
                <Square class="mr-2 size-4" />
                Stop
            </Button>
        </div>
    </div>

    <div class="flex-1 border-t">
        <Resizable.PaneGroup direction="vertical">
            <Resizable.Pane defaultSize={70}>
                <!-- We give CodeMirror full height -->
                <div
                    class="h-full w-full overflow-hidden [&_.cm-editor]:h-full [&_.cm-scroller]:font-mono [&_.cm-scroller]:text-sm"
                >
                    <CodeMirror
                        bind:value={code}
                        lang={yaml()}
                        styles={{
                            "&": {
                                height: "100%",
                            },
                        }}
                    />
                </div>
            </Resizable.Pane>
            <Resizable.Handle withHandle />
            <Resizable.Pane defaultSize={30}>
                <div class="h-full flex flex-col bg-slate-950 text-slate-50">
                    <div
                        class="flex items-center gap-2 px-3 py-1.5 border-b border-slate-800 bg-slate-900 text-xs font-mono text-slate-400"
                    >
                        <ActivitySquare class="size-3.5" />
                        Output Logs
                    </div>
                    <ScrollArea class="flex-1 p-3">
                        <div class="font-mono text-xs space-y-1">
                            {#if outputLogs.length === 0}
                                <div class="text-slate-500 italic">
                                    No output yet. Run a dataflow to see logs.
                                </div>
                            {/if}
                            {#each outputLogs as log}
                                <div
                                    class={log.includes("ERROR")
                                        ? "text-red-400"
                                        : "text-slate-300"}
                                >
                                    {log}
                                </div>
                            {/each}
                        </div>
                    </ScrollArea>
                </div>
            </Resizable.Pane>
        </Resizable.PaneGroup>
    </div>
</div>
