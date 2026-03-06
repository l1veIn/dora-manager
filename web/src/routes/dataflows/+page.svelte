<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { get, post } from "$lib/api";
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import {
        Code,
        Plus,
        Search,
        Pencil,
        Trash2,
        Save,
        ArrowLeft,
        FileText,
        Play,
        Square,
        Rocket,
        Activity,
    } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import CodeMirror from "svelte-codemirror-editor";
    import { yaml } from "@codemirror/lang-yaml";
    import DataflowRunActions from "$lib/components/dataflows/DataflowRunActions.svelte";

    let dataflows = $state<any[]>([]);
    let loading = $state(true);
    let searchQuery = $state("");

    // Active runs mapping: dataflowName -> runId
    let activeRuns = $state<Record<string, string>>({});
    let runsPolling: ReturnType<typeof setInterval> | null = null;

    // Dialog state
    let isCreateDialogOpen = $state(false);
    let newDataflowName = $state("");
    let isDeleteDialogOpen = $state(false);
    let dataflowToDelete = $state("");

    // Editor state
    let editingName = $derived(page.url.searchParams.get("edit"));
    let code = $state("");
    let isStarting = $state(false);
    let isSaving = $state(false);

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

    async function fetchActiveRuns() {
        try {
            const result = (await get(`/runs/active`)) as any;
            const runs = Array.isArray(result) ? result : result.runs || [];
            const newMapping: Record<string, string> = {};
            for (const r of runs) {
                newMapping[r.name] = r.id;
            }
            activeRuns = newMapping;
        } catch (e) {
            console.error("Failed to fetch active runs", e);
        }
    }

    onMount(() => {
        fetchDataflows();
        fetchActiveRuns();
        runsPolling = setInterval(fetchActiveRuns, 3000);
    });

    onDestroy(() => {
        if (runsPolling) clearInterval(runsPolling);
    });

    $effect(() => {
        if (editingName) {
            loadDataflow(editingName);
        } else {
            code = "";
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

    function openCreateDialog() {
        newDataflowName = "";
        isCreateDialogOpen = true;
    }

    async function confirmCreateDataflow() {
        const safeName = newDataflowName.replace(/[^a-zA-Z0-9_-]/g, "");
        if (!safeName) {
            toast.error("Invalid name.");
            return;
        }
        isCreateDialogOpen = false;
        try {
            const initialYaml = `nodes:\n  - id: custom_node\n    operator:\n      python: |\n        def process(event, state):\n            return event\n`;
            await post(`/dataflows/${safeName}`, { yaml: initialYaml });
            goto(`/dataflows?edit=${safeName}`);
        } catch (e: any) {
            toast.error(`Failed to create: ${e.message}`);
        }
    }

    function openDeleteDialog(name: string) {
        dataflowToDelete = name;
        isDeleteDialogOpen = true;
    }

    async function confirmDeleteDataflow() {
        if (!dataflowToDelete) return;
        const name = dataflowToDelete;
        isDeleteDialogOpen = false;
        try {
            await post(`/dataflows/${name}/delete`, {});
            toast.success(`${name} deleted`);
            fetchDataflows();
        } catch (e: any) {
            toast.error(`Delete failed: ${e.message}`);
        }
    }

    async function runDataflowFromList(name: string, force = false) {
        if (activeRuns[name]) {
            goto(`/runs/${activeRuns[name]}`);
            return;
        }
        try {
            const res: any = await get(`/dataflows/${name}`);
            const result: any = await post("/runs/start", {
                yaml: res.yaml,
                name,
                force,
            });
            toast.success(`Started ${name}`);
            if (result.run_id) goto(`/runs/${result.run_id}`);
        } catch (e: any) {
            toast.error(`Run failed: ${e.message}`);
        }
    }

    async function stopActiveRun(runId: string) {
        try {
            await post(`/runs/${runId}/stop`, {});
            toast.success("Run stopped");
            fetchActiveRuns();
        } catch (e: any) {
            toast.error(`Stop failed: ${e.message}`);
        }
    }

    async function runEditorDataflow(force: boolean = false) {
        if (isStarting || !editingName) return;
        await saveDataflow();
        isStarting = true;
        try {
            const res: any = await post("/runs/start", {
                yaml: code,
                name: editingName,
                force,
            });
            toast.success(res.message);
            if (res.run_id) {
                goto(`/runs/${res.run_id}`);
            }
        } catch (e: any) {
            toast.error(`Run failed: ${e.message}`);
        } finally {
            isStarting = false;
        }
    }

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
            <Button onclick={openCreateDialog}>
                <Plus class="size-4 mr-2" />
                New Dataflow
            </Button>
        </div>

        <div class="flex items-center justify-between">
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
                    onclick={openCreateDialog}
                >
                    <Plus class="size-4 mr-2" /> New Dataflow
                </Button>
            </div>
        {:else}
            <div class="grid gap-4">
                {#each filteredDataflows as df}
                    {@const isActive = !!activeRuns[df.name]}
                    {@const runId = activeRuns[df.name]}
                    <div
                        class="flex items-center justify-between p-4 border rounded-lg bg-card hover:border-slate-300 dark:hover:border-slate-700 transition-colors"
                    >
                        <div class="flex items-center gap-4">
                            <div
                                class="p-2 bg-primary/10 rounded-md text-primary relative"
                            >
                                {#if isActive}
                                    <span
                                        class="absolute -top-1 -right-1 flex h-3 w-3"
                                    >
                                        <span
                                            class="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"
                                        ></span>
                                        <span
                                            class="relative inline-flex rounded-full h-3 w-3 bg-blue-500"
                                        ></span>
                                    </span>
                                {/if}
                                <FileText class="size-5" />
                            </div>
                            <div class="flex flex-col gap-0.5">
                                <div class="flex items-center gap-2">
                                    <h3 class="font-medium font-mono">
                                        {df.filename}
                                    </h3>
                                    {#if isActive}
                                        <Badge
                                            variant="outline"
                                            class="font-mono text-[10px] bg-blue-50 text-blue-700 border-blue-200 hover:bg-blue-100 cursor-pointer"
                                            onclick={() =>
                                                goto(`/runs/${runId}`)}
                                        >
                                            <Activity class="size-3 mr-1" /> Active
                                        </Badge>
                                    {/if}
                                </div>
                                <div class="text-sm text-muted-foreground">
                                    Modified: {new Date(
                                        df.modified_at,
                                    ).toLocaleString()} &middot; {df.size} bytes
                                </div>
                            </div>
                        </div>
                        <div class="flex items-center gap-2">
                            {#if isActive}
                                <Button
                                    variant="destructive"
                                    size="sm"
                                    onclick={() => stopActiveRun(runId)}
                                >
                                    <Square class="size-4 mr-1" /> Stop
                                </Button>
                            {:else}
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onclick={() => runDataflowFromList(df.name)}
                                >
                                    <Play class="size-4 mr-1" /> Run
                                </Button>
                            {/if}
                            <Button
                                variant="outline"
                                size="sm"
                                onclick={() =>
                                    goto(`/dataflows?edit=${df.name}`)}
                            >
                                <Pencil class="size-4 mr-1" /> Edit
                            </Button>
                            <Button
                                variant="ghost"
                                size="sm"
                                class="text-red-500 hover:text-red-600 hover:bg-red-500/10"
                                onclick={() => openDeleteDialog(df.name)}
                            >
                                <Trash2 class="size-4" />
                            </Button>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}

        <Dialog.Root bind:open={isCreateDialogOpen}>
            <Dialog.Content class="sm:max-w-[425px]">
                <Dialog.Header>
                    <Dialog.Title>Create Dataflow</Dialog.Title>
                </Dialog.Header>
                <div class="grid gap-4 py-4">
                    <Input
                        bind:value={newDataflowName}
                        placeholder="my-dataflow"
                        autofocus
                        onkeydown={(e) =>
                            e.key === "Enter" && confirmCreateDataflow()}
                    />
                </div>
                <Dialog.Footer>
                    <Button
                        variant="outline"
                        onclick={() => (isCreateDialogOpen = false)}
                        >Cancel</Button
                    >
                    <Button type="submit" onclick={confirmCreateDataflow}
                        >Create</Button
                    >
                </Dialog.Footer>
            </Dialog.Content>
        </Dialog.Root>

        <AlertDialog.Root bind:open={isDeleteDialogOpen}>
            <AlertDialog.Content>
                <AlertDialog.Header>
                    <AlertDialog.Title>Are you sure?</AlertDialog.Title>
                    <AlertDialog.Description
                        >This action cannot be undone. This will permanently
                        delete the dataflow.</AlertDialog.Description
                    >
                </AlertDialog.Header>
                <AlertDialog.Footer>
                    <AlertDialog.Cancel
                        onclick={() => (isDeleteDialogOpen = false)}
                        >Cancel</AlertDialog.Cancel
                    >
                    <AlertDialog.Action
                        onclick={confirmDeleteDataflow}
                        class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                        >Delete</AlertDialog.Action
                    >
                </AlertDialog.Footer>
            </AlertDialog.Content>
        </AlertDialog.Root>
    </div>
{:else}
    <!-- EDITOR VIEW -->
    <div
        class="h-full flex flex-col pt-4 max-w-7xl mx-auto w-[calc(100%-2rem)]"
    >
        <div
            class="flex items-center justify-between px-6 pb-4 shrink-0 border-b"
        >
            <div class="flex items-center gap-4">
                <Button
                    variant="ghost"
                    size="icon"
                    onclick={() => goto("/dataflows")}
                >
                    <ArrowLeft class="size-5" />
                </Button>
                <div class="flex items-center gap-3">
                    <div class="p-1.5 bg-primary/10 rounded-md">
                        <Code class="size-4 text-primary" />
                    </div>
                    <div>
                        <h1
                            class="text-xl font-bold tracking-tight font-mono inline-flex items-center gap-2"
                        >
                            {editingName}.yml
                            {#if activeRuns[editingName]}
                                <Badge
                                    variant="outline"
                                    class="font-mono text-[10px] bg-blue-50 text-blue-700 border-blue-200 ml-2 cursor-pointer"
                                    onclick={() =>
                                        goto(
                                            `/runs/${activeRuns[editingName]}`,
                                        )}
                                >
                                    <Activity class="size-3 mr-1" /> Active Output
                                </Badge>
                            {/if}
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

                <DataflowRunActions
                    activeRunId={activeRuns[editingName]}
                    {isStarting}
                    onRun={runEditorDataflow}
                    onStop={() => stopActiveRun(activeRuns[editingName])}
                    onViewRun={() => goto(`/runs/${activeRuns[editingName]}`)}
                />
            </div>
        </div>

        <div class="flex-1 min-h-0 bg-background pt-2 pb-6 px-1">
            <div
                class="h-full w-full border rounded-lg shadow-sm overflow-y-auto [&_.cm-editor]:h-full [&_.cm-scroller]:font-mono [&_.cm-scroller]:text-[13px]"
            >
                <CodeMirror
                    bind:value={code}
                    lang={yaml()}
                    styles={{
                        "&": {
                            height: "100%",
                            width: "100%",
                        },
                    }}
                />
            </div>
        </div>
    </div>
{/if}
