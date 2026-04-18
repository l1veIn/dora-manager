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
    let mediaStatus = $state<any>(null);

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
            const [dataflowList, status] = (await Promise.all([
                get("/dataflows"),
                get("/media/status").catch(() => null),
            ])) as [any[], any];
            dataflows = dataflowList || [];
            mediaStatus = status;
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
        const dataflowMeta = dataflows.find((item) => item.name === name);
        if (mediaBlocked(dataflowMeta?.executable)) {
            toast.error(
                "This dataflow requires dm-server media support. Open Settings > Media first.",
            );
            return;
        }

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
                view_json: res.view ? JSON.stringify(res.view) : undefined,
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

    const recommendedDataflowNames = new Set([
        "demo-hello-timer",
        "interaction-demo",
    ]);

    function isRecommended(df: any) {
        return recommendedDataflowNames.has(df.name);
    }

    function needsAttention(df: any) {
        return !df.executable?.can_run || mediaBlocked(df.executable);
    }

    function statusLabel(df: any) {
        if (df.executable?.invalid_yaml) return "Invalid YAML";
        if (df.executable?.missing_nodes?.length) return "Missing Nodes";
        if (mediaBlocked(df.executable)) return "Media Required";
        return "Ready";
    }

    function statusBadgeClass(df: any) {
        if (df.executable?.invalid_yaml || df.executable?.missing_nodes?.length) {
            return "font-mono text-[10px] bg-red-50 text-red-700 border-red-200";
        }
        if (mediaBlocked(df.executable)) {
            return "font-mono text-[10px] bg-amber-50 text-amber-800 border-amber-200";
        }
        return "font-mono text-[10px] bg-green-50 text-green-700 border-green-200";
    }

    let primaryDataflows = $derived(
        [...filteredDataflows]
            .filter((df) => !needsAttention(df))
            .sort((a, b) => Number(isRecommended(b)) - Number(isRecommended(a))),
    );

    let attentionDataflows = $derived(
        filteredDataflows.filter((df) => needsAttention(df)),
    );

    function mediaBlocked(executable: any) {
        return (
            executable?.requires_media_backend &&
            mediaStatus?.status !== "ready"
        );
    }
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

    <div class="rounded-xl border bg-muted/20 p-4 md:p-5">
        <div class="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
            <div class="space-y-2">
                <p class="text-sm font-medium">
                    This page is your saved workspace map.
                </p>
                <p class="text-sm text-muted-foreground max-w-3xl">
                    Use <span class="font-medium text-foreground">Run</span> to
                    start a fresh run from the saved workspace. Use <span
                        class="font-medium text-foreground">Workspace</span
                    > to inspect or edit the persistent YAML and graph.
                </p>
                <p class="text-sm text-muted-foreground max-w-3xl">
                    If you are deciding where to go after the first demo, start
                    with <span class="font-mono text-foreground"
                        >demo-hello-timer</span
                    > or <span class="font-mono text-foreground"
                        >interaction-demo</span
                    >. Items that need fixes or extra setup are grouped below.
                </p>
            </div>
        </div>
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
        <div class="space-y-6">
            {#if primaryDataflows.length}
                <section class="space-y-3">
                    <div class="space-y-1">
                        <h2 class="text-sm font-semibold tracking-wide uppercase text-muted-foreground">
                            Ready To Explore
                        </h2>
                        <p class="text-sm text-muted-foreground">
                            These workspaces are runnable now and safe entry
                            points for your next task.
                        </p>
                    </div>
                    <div class="grid gap-4">
                        {#each primaryDataflows as df}
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
                                        <div class="flex items-center gap-2 flex-wrap">
                                            <a
                                                class="font-medium font-mono text-lg hover:underline cursor-pointer"
                                                href={`/dataflows/${df.name}`}
                                            >
                                                {df.meta?.name || df.name}
                                            </a>
                                            {#if isRecommended(df)}
                                                <Badge
                                                    variant="outline"
                                                    class="font-mono text-[10px] bg-sky-50 text-sky-700 border-sky-200"
                                                >
                                                    Recommended
                                                </Badge>
                                            {/if}
                                            <Badge
                                                variant="outline"
                                                class={statusBadgeClass(df)}
                                            >
                                                {statusLabel(df)}
                                            </Badge>
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
                                        </div>
                                        <div
                                            class="text-sm text-muted-foreground flex gap-2 items-center mt-1 flex-wrap"
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
                                                    >{df.executable.resolved_node_count}
                                                    nodes</span
                                                >
                                            {/if}
                                        </div>
                                    </div>
                                </div>
                                <div class="flex items-center gap-2">
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        disabled={(df.executable && !df.executable.can_run) || mediaBlocked(df.executable)}
                                        onclick={() => runDataflowFromList(df.name)}
                                        title={mediaBlocked(df.executable)
                                            ? "This dataflow needs dm-server media support. Open Settings > Media."
                                            : ""}
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
                </section>
            {/if}

            {#if attentionDataflows.length}
                <section class="space-y-3">
                    <div class="space-y-1">
                        <h2 class="text-sm font-semibold tracking-wide uppercase text-muted-foreground">
                            Needs Attention Or Extra Setup
                        </h2>
                        <p class="text-sm text-muted-foreground">
                            These workspaces are still useful, but they need
                            missing nodes restored, YAML fixed, or media support
                            configured before they can run.
                        </p>
                    </div>
                    <div class="grid gap-4">
                        {#each attentionDataflows as df}
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
                                        <div class="flex items-center gap-2 flex-wrap">
                                            <a
                                                class="font-medium font-mono text-lg hover:underline cursor-pointer"
                                                href={`/dataflows/${df.name}`}
                                            >
                                                {df.meta?.name || df.name}
                                            </a>
                                            <Badge
                                                variant="outline"
                                                class={statusBadgeClass(df)}
                                            >
                                                {statusLabel(df)}
                                            </Badge>
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
                                        </div>
                                        <div
                                            class="text-sm text-muted-foreground flex gap-2 items-center mt-1 flex-wrap"
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
                                                    >{df.executable.resolved_node_count}
                                                    nodes</span
                                                >
                                            {/if}
                                            {#if mediaBlocked(df.executable)}
                                                &middot;
                                                <span class="text-amber-700">
                                                    Configure media backend in Settings
                                                </span>
                                            {/if}
                                            {#if df.executable?.missing_nodes?.length}
                                                &middot;
                                                <span class="text-red-700">
                                                    Restore {df.executable.missing_nodes.join(", ")}
                                                </span>
                                            {/if}
                                        </div>
                                    </div>
                                </div>
                                <div class="flex items-center gap-2">
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        disabled={(df.executable && !df.executable.can_run) || mediaBlocked(df.executable)}
                                        onclick={() => runDataflowFromList(df.name)}
                                        title={mediaBlocked(df.executable)
                                            ? "This dataflow needs dm-server media support. Open Settings > Media."
                                            : ""}
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
                </section>
            {/if}
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
