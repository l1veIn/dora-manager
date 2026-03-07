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
        Plus,
        Search,
        Pencil,
        Trash2,
        ArrowLeft,
        FileText,
        Play,
        Rocket,
        AlertCircle,
        AlertTriangle,
    } from "lucide-svelte";
    import { toast } from "svelte-sonner";

    let dataflows = $state<any[]>([]);
    let loading = $state(true);
    let searchQuery = $state("");

    // Dialog state
    let isCreateDialogOpen = $state(false);
    let newDataflowName = $state("");
    let isDeleteDialogOpen = $state(false);
    let dataflowToDelete = $state("");
    let isRunConflictDialogOpen = $state(false);
    let dataflowToRun = $state("");

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

    $effect(() => {
        fetchDataflows();
    });

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
            goto(`/dataflows/${safeName}`);
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
        if (!force) {
            try {
                const result = (await get(`/runs/active`)) as any;
                const runs = Array.isArray(result) ? result : result.runs || [];
                const isActive = runs.some((r: any) => r.name === name);

                if (isActive) {
                    dataflowToRun = name;
                    isRunConflictDialogOpen = true;
                    return;
                }
            } catch (e) {
                console.error(
                    "Failed to check active runs during pre-flight check",
                    e,
                );
            }
        }

        try {
            isRunConflictDialogOpen = false;
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

    let filteredDataflows = $derived(
        dataflows.filter((d) =>
            (d.meta?.name || d.name)
                .toLowerCase()
                .includes(searchQuery.toLowerCase()),
        ),
    );
</script>

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
                <div class="animate-pulse h-20 bg-muted/50 rounded-lg"></div>
            {/each}
        </div>
    {:else if filteredDataflows.length === 0}
        <div
            class="flex flex-col items-center justify-center p-12 text-center border rounded-lg bg-muted/10 h-64 border-dashed"
        >
            <FileText class="h-12 w-12 text-muted-foreground mb-4 opacity-50" />
            <h3 class="text-lg font-medium">No dataflows found</h3>
            <p class="text-sm text-muted-foreground mt-1 max-w-sm">
                Create your first dataflow to start processing data.
            </p>
            <Button variant="outline" class="mt-4" onclick={openCreateDialog}>
                <Plus class="size-4 mr-2" /> New Dataflow
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
                            class="p-2 bg-primary/10 rounded-md text-primary relative"
                        >
                            <FileText class="size-5" />
                        </div>
                        <div class="flex flex-col gap-0.5">
                            <div class="flex items-center gap-2">
                                <a
                                    class="font-medium font-mono text-lg hover:underline cursor-pointer"
                                    href={`/dataflows/${df.name}`}
                                >
                                    {df.meta?.name || df.name}
                                </a>
                                {#if df.meta?.tags?.length}
                                    <div class="flex gap-1">
                                        {#each df.meta.tags as tag}
                                            <Badge
                                                variant="secondary"
                                                class="text-[10px] px-1 py-0"
                                                >{tag}</Badge
                                            >
                                        {/each}
                                    </div>
                                {/if}
                                {#if df.executable && !df.executable.can_run}
                                    <Badge
                                        variant="outline"
                                        class="font-mono text-[10px] bg-red-50 text-red-700 border-red-200"
                                    >
                                        {#if df.executable.invalid_yaml}
                                            <AlertCircle class="size-3 mr-1" /> Invalid
                                            YAML
                                        {:else if df.executable.missing_nodes?.length}
                                            <AlertTriangle
                                                class="size-3 mr-1"
                                            /> Missing Nodes
                                        {/if}
                                    </Badge>
                                {/if}
                            </div>
                            <div
                                class="text-sm text-muted-foreground flex gap-2 items-center mt-1"
                            >
                                <span class="font-mono text-xs">{df.name}</span>
                                &middot;
                                <span
                                    >Modified: {new Date(
                                        df.modified_at,
                                    ).toLocaleString()}</span
                                >
                                {#if df.executable && df.executable.resolved_node_count > 0}
                                    &middot; <span
                                        >{df.executable.resolved_node_count} nodes</span
                                    >
                                {/if}
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center gap-2">
                        <Button
                            variant="outline"
                            size="sm"
                            disabled={df.executable && !df.executable.can_run}
                            onclick={() => runDataflowFromList(df.name)}
                        >
                            <Play class="size-4 mr-1" /> Run
                        </Button>
                        <Button
                            variant="secondary"
                            size="sm"
                            onclick={() => goto(`/dataflows/${df.name}`)}
                        >
                            <Pencil class="size-4 mr-1" /> Workspace
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
                    onclick={() => (isCreateDialogOpen = false)}>Cancel</Button
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
                    >This action cannot be undone. This will permanently delete
                    the dataflow.</AlertDialog.Description
                >
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel onclick={() => (isDeleteDialogOpen = false)}
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

    <AlertDialog.Root bind:open={isRunConflictDialogOpen}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>Run Already Active</AlertDialog.Title>
                <AlertDialog.Description>
                    There is already an active instance of <span
                        class="font-mono font-medium text-foreground"
                        >{dataflowToRun}</span
                    > running. Are you sure you want to spawn another concurrent
                    instance?
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel
                    onclick={() => (isRunConflictDialogOpen = false)}
                    >Cancel</AlertDialog.Cancel
                >
                <AlertDialog.Action
                    onclick={() => runDataflowFromList(dataflowToRun, true)}
                    >Run Anyway</AlertDialog.Action
                >
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>
</div>
