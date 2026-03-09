<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import { goto } from "$app/navigation";
    import { toast } from "svelte-sonner";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Skeleton } from "$lib/components/ui/skeleton/index.js";
    import {
        ArrowLeft,
        Play,
        Square,
        AlertTriangle,
        AlertCircle,
        Code,
        Settings,
        FileText,
        History,
    } from "lucide-svelte";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import YamlEditorTab from "./components/YamlEditorTab.svelte";
    import MetaTab from "./components/MetaTab.svelte";
    import ConfigOverridesTab from "./components/ConfigOverridesTab.svelte";
    import HistoryTab from "./components/HistoryTab.svelte";

    let dataflowName = $derived(page.params.id as string);
    let dataflow = $state<any>(null);
    let loading = $state(true);
    let isRunConflictDialogOpen = $state(false);

    async function loadDataflow() {
        loading = true;
        try {
            const res = await get(`/dataflows/${dataflowName}`);
            dataflow = res;
        } catch (e: any) {
            toast.error(`Failed to load workspace: ${e.message}`);
            goto("/dataflows");
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadDataflow();
    });

    async function handleRun(force = false) {
        if (!dataflow?.executable?.can_run) return;

        if (!force) {
            try {
                const result = (await get(`/runs/active`)) as any;
                const runs = Array.isArray(result) ? result : result.runs || [];
                const isActive = runs.some((r: any) => r.name === dataflowName);

                if (isActive) {
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
            const res: any = await post("/runs/start", {
                name: dataflowName,
                yaml: dataflow.yaml,
                force,
            });
            toast.success("Started dataflow");
            if (res.run_id) {
                goto(`/runs/${res.run_id}`);
            }
        } catch (e: any) {
            toast.error(`Run failed: ${e.message}`);
        }
    }
</script>

<div
    class="flex flex-col h-full pt-6 pb-6 px-4 md:px-8 w-full max-w-7xl mx-auto space-y-6 min-h-0"
>
    <!-- Header -->
    <div class="flex items-start justify-between">
        <div class="flex flex-col gap-1">
            <Button
                variant="ghost"
                size="sm"
                class="w-fit -ml-2 text-muted-foreground hover:text-foreground mb-2"
                href="/dataflows"
            >
                <ArrowLeft class="size-4 mr-1" />
                Back to Dataflows
            </Button>
            <div class="flex items-center gap-3">
                <h1
                    class="text-3xl font-bold font-mono tracking-tight flex items-center gap-2"
                >
                    <FileText class="size-6 text-primary" />
                    {dataflow?.meta?.name || dataflowName}
                </h1>
                {#if dataflow?.executable && !dataflow.executable.can_run}
                    <Badge
                        variant="outline"
                        class="bg-red-50 text-red-700 border-red-200"
                    >
                        {#if dataflow.executable.invalid_yaml}
                            <AlertCircle class="size-3 mr-1" /> Invalid YAML
                        {:else if dataflow.executable.missing_nodes?.length}
                            <AlertTriangle class="size-3 mr-1" /> Missing Nodes
                        {/if}
                    </Badge>
                {:else if dataflow?.executable?.can_run}
                    <Badge
                        variant="outline"
                        class="bg-green-50 text-green-700 border-green-200"
                    >
                        Ready
                    </Badge>
                {/if}
            </div>
            {#if dataflow?.meta?.description}
                <p class="text-muted-foreground mt-1 max-w-2xl text-sm">
                    {dataflow.meta.description}
                </p>
            {/if}
        </div>

        {#if dataflow}
            <div class="flex items-center gap-2">
                <Button
                    class="gap-2"
                    disabled={!dataflow?.executable?.can_run}
                    onclick={() => handleRun()}
                >
                    <Play class="size-4" /> Run
                </Button>
            </div>
        {/if}
    </div>

    {#if loading}
        <div class="space-y-4">
            <Skeleton class="h-10 w-full max-w-md" />
            <Skeleton class="h-[60vh] w-full rounded-lg" />
        </div>
    {:else if dataflow}
        <!-- Workspace Main Content -->
        <Tabs.Root value="yaml" class="flex-1 flex flex-col min-h-0">
            <Tabs.List class="w-fit mb-4">
                <Tabs.Trigger value="yaml" class="gap-2">
                    <Code class="size-4" />
                    dataflow.yml
                </Tabs.Trigger>
                <Tabs.Trigger value="meta" class="gap-2">
                    <FileText class="size-4" />
                    flow.json
                </Tabs.Trigger>
                <Tabs.Trigger value="config" class="gap-2">
                    <Settings class="size-4" />
                    Config Overrides
                </Tabs.Trigger>
                <Tabs.Trigger value="history" class="gap-2">
                    <History class="size-4" />
                    History
                </Tabs.Trigger>
            </Tabs.List>

            <Tabs.Content
                value="yaml"
                class="flex-1 flex flex-col min-h-0 overflow-hidden mt-0"
            >
                {#if dataflow?.yaml !== undefined}
                    <YamlEditorTab
                        {dataflowName}
                        initialYaml={dataflow.yaml || ""}
                        onCodeUpdated={(newYaml) => {
                            if (dataflow) dataflow.yaml = newYaml;
                        }}
                    />
                {/if}
            </Tabs.Content>

            <Tabs.Content
                value="config"
                class="flex-1 border rounded-md bg-card shadow-sm flex flex-col min-h-0 overflow-hidden mt-0"
            >
                <ConfigOverridesTab {dataflowName} />
            </Tabs.Content>

            <Tabs.Content
                value="meta"
                class="flex-1 border rounded-md bg-card shadow-sm flex flex-col min-h-0 overflow-hidden mt-0"
            >
                {#if dataflow?.meta !== undefined}
                    <MetaTab
                        {dataflowName}
                        meta={dataflow.meta}
                        onMetaUpdated={(newMeta) => {
                            if (dataflow) dataflow.meta = newMeta;
                        }}
                    />
                {/if}
            </Tabs.Content>

            <Tabs.Content
                value="history"
                class="flex-1 flex flex-col min-h-0 overflow-hidden mt-0"
            >
                <HistoryTab {dataflowName} />
            </Tabs.Content>
        </Tabs.Root>
    {/if}

    <AlertDialog.Root bind:open={isRunConflictDialogOpen}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>Run Already Active</AlertDialog.Title>
                <AlertDialog.Description>
                    There is already an active instance of <span
                        class="font-mono font-medium text-foreground"
                        >{dataflowName}</span
                    > running. Are you sure you want to spawn another concurrent
                    instance?
                </AlertDialog.Description>
            </AlertDialog.Header>
            <AlertDialog.Footer>
                <AlertDialog.Cancel
                    onclick={() => (isRunConflictDialogOpen = false)}
                    >Cancel</AlertDialog.Cancel
                >
                <AlertDialog.Action onclick={() => handleRun(true)}
                    >Run Anyway</AlertDialog.Action
                >
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>
</div>
