<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { get, post } from "$lib/api";
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import * as Resizable from "$lib/components/ui/resizable/index.js";
    import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
    import {
        Play,
        Square,
        Code,
        ActivitySquare,
        Plus,
        Search,
        Pencil,
        Trash2,
        Save,
        ArrowLeft,
        FileText,
    } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import CodeMirror from "svelte-codemirror-editor";
    import { yaml } from "@codemirror/lang-yaml";

    let dataflows = $state<any[]>([]);
    let loading = $state(true);
    let searchQuery = $state("");

    // Editor state
    let editingName = $derived(page.url.searchParams.get("edit"));
    let code = $state("");
    let isRunning = $state(false);
    let isSaving = $state(false);
    let outputLogs = $state<string[]>([]);
    let monitorInterval: ReturnType<typeof setInterval>;

    async function fetchDataflows() {
        loading = true;
        try {
            dataflows = (await get("/dataflows")) || [];
        } catch (e: any) {
            toast.error(`Failed to load dataflows: ${e.message}`);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        fetchDataflows();
    });

    // Handle Editor View loading
    $effect(() => {
        if (editingName) {
            loadDataflow(editingName);
        } else {
            // clear editor state when returning to list
            code = "";
            outputLogs = [];
            stopMonitoring();
            fetchDataflows();
        }
    });

    async function loadDataflow(name: string) {
        try {
            const res: any = await get(`/dataflows/${name}`);
            code = res.yaml;
        } catch (e: any) {
            toast.error(`Failed to load ${name}.yml: ${e.message}`);
            goto("/dataflows");
        }
    }

    async function saveDataflow() {
        if (!editingName) return;
        isSaving = true;
        try {
            await post(`/dataflows/${editingName}`, { yaml: code });
            toast.success("Saved successfully");
            fetchDataflows();
        } catch (e: any) {
            toast.error(`Save failed: ${e.message}`);
        } finally {
            isSaving = false;
        }
    }

    async function createNewDataflow() {
        const name = prompt("Enter a name for the new dataflow:");
        if (!name) return;

        const safeName = name.replace(/[^a-zA-Z0-9_-]/g, "");
        if (!safeName) {
            toast.error(
                "Invalid name. Use alphanumeric characters, dashes, and underscores.",
            );
            return;
        }

        try {
            const initialYaml = `nodes:\n  - id: custom_node\n    operator:\n      python: |\n        def process(event, state):\n            return event\n`;
            await post(`/dataflows/${safeName}`, { yaml: initialYaml });
            goto(`/dataflows?edit=${safeName}`);
        } catch (e: any) {
            toast.error(`Failed to create: ${e.message}`);
        }
    }

    async function deleteDataflow(name: string) {
        if (!confirm(`Are you sure you want to delete ${name}.yml?`)) return;
        try {
            await post(`/dataflows/${name}/delete`, {});
            toast.success(`${name} deleted`);
            fetchDataflows();
        } catch (e: any) {
            toast.error(`Delete failed: ${e.message}`);
        }
    }

    async function runDataflowFromList(name: string) {
        try {
            const res: any = await get(`/dataflows/${name}`);
            await post("/dataflow/run", { yaml: res.yaml });
            toast.success(`Started ${name}`);
        } catch (e: any) {
            toast.error(`Run failed: ${e.message}`);
        }
    }

    async function runDataflow() {
        if (isRunning || !editingName) return;
        await saveDataflow(); // Autosave before run
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
            const res: any = await post("/dataflow/stop", {});
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

    let lastEventId = 0;

    function startMonitoring() {
        stopMonitoring();
        lastEventId = 0;
        monitorInterval = setInterval(async () => {
            try {
                const events: any[] = await get(
                    `/events?source=dataflow&limit=50`,
                );
                if (events && events.length > 0) {
                    const newEvents = events
                        .filter((ev) => ev.id > lastEventId)
                        .reverse();
                    if (newEvents.length > 0) {
                        lastEventId = Math.max(...events.map((ev) => ev.id));
                        newEvents.forEach((ev) => {
                            const logStr = `[${new Date(ev.timestamp).toLocaleTimeString()}] [${ev.source}] ${ev.activity || ev.message || ""}`;
                            outputLogs = [...outputLogs, logStr].slice(-200);
                        });
                    }
                }
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

    let filteredDataflows = $derived(
        dataflows.filter((d) =>
            d.filename.toLowerCase().includes(searchQuery.toLowerCase()),
        ),
    );
</script>

{#if !editingName}
    <!-- LIST VIEW -->
    <div class="h-full flex flex-col p-6 max-w-6xl mx-auto space-y-6 w-full">
        <div class="flex items-center justify-between">
            <h1 class="text-3xl font-bold tracking-tight">Dataflows</h1>
            <Button onclick={createNewDataflow}>
                <Plus class="size-4 mr-2" />
                New Dataflow
            </Button>
        </div>

        <div class="flex items-center">
            <div class="relative w-80">
                <Search
                    class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground"
                />
                <Input
                    type="search"
                    placeholder="Search dataflows..."
                    class="pl-8"
                    bind:value={searchQuery}
                />
            </div>
        </div>

        {#if loading}
            <div class="space-y-4">
                {#each Array(3) as _}
                    <div
                        class="animate-pulse h-20 bg-muted/50 rounded-lg"
                    ></div>
                {/each}
            </div>
        {:else if filteredDataflows.length === 0}
            <div
                class="flex flex-col items-center justify-center p-12 text-center border rounded-lg bg-muted/10 h-64 border-dashed"
            >
                <FileText
                    class="h-12 w-12 text-muted-foreground mb-4 opacity-50"
                />
                <h3 class="text-lg font-medium">No dataflows found</h3>
                <p class="text-sm text-muted-foreground mt-1 max-w-sm">
                    Create your first dataflow to start processing data.
                </p>
                <Button
                    variant="outline"
                    class="mt-4"
                    onclick={createNewDataflow}
                >
                    <Plus class="size-4 mr-2" />
                    New Dataflow
                </Button>
            </div>
        {:else}
            <div class="grid gap-4">
                {#each filteredDataflows as df}
                    <div
                        class="flex items-center justify-between p-4 border rounded-lg bg-card hover:border-slate-300 dark:hover:border-slate-700 transition-colors"
                    >
                        <div class="flex items-center gap-4">
                            <div
                                class="p-2 bg-primary/10 rounded-md text-primary"
                            >
                                <FileText class="size-5" />
                            </div>
                            <div>
                                <h3 class="font-medium font-mono">
                                    {df.filename}
                                </h3>
                                <div class="text-sm text-muted-foreground">
                                    Modified: {new Date(
                                        df.modified_at,
                                    ).toLocaleString()} &middot; {df.size} bytes
                                </div>
                            </div>
                        </div>
                        <div class="flex items-center gap-2">
                            <Button
                                variant="outline"
                                size="sm"
                                onclick={() => runDataflowFromList(df.name)}
                            >
                                <Play class="size-4 mr-1" />
                                Run
                            </Button>
                            <Button
                                variant="outline"
                                size="sm"
                                onclick={() =>
                                    goto(`/dataflows?edit=${df.name}`)}
                            >
                                <Pencil class="size-4 mr-1" />
                                Edit
                            </Button>
                            <Button
                                variant="ghost"
                                size="sm"
                                class="text-red-500 hover:text-red-600 hover:bg-red-500/10"
                                onclick={() => deleteDataflow(df.name)}
                            >
                                <Trash2 class="size-4" />
                            </Button>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
{:else}
    <!-- EDITOR VIEW -->
    <div class="h-full flex flex-col pt-4">
        <div class="flex items-center justify-between px-6 pb-4">
            <div class="flex items-center gap-4">
                <Button
                    variant="ghost"
                    size="icon"
                    onclick={() => goto("/dataflows")}
                >
                    <ArrowLeft class="size-5" />
                </Button>
                <div class="flex items-center gap-2">
                    <div class="p-1.5 bg-primary/10 rounded-md">
                        <Code class="size-4 text-primary" />
                    </div>
                    <div>
                        <h1 class="text-xl font-bold tracking-tight font-mono">
                            {editingName}.yml
                        </h1>
                    </div>
                </div>
            </div>

            <div class="flex items-center gap-2">
                <Button
                    variant="outline"
                    size="sm"
                    disabled={isSaving}
                    onclick={saveDataflow}
                >
                    <Save class="mr-2 size-4" />
                    {isSaving ? "Saving..." : "Save"}
                </Button>
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
                    <div
                        class="h-full flex flex-col bg-slate-950 text-slate-50"
                    >
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
                                        No output yet. Run a dataflow to see
                                        logs.
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
{/if}
