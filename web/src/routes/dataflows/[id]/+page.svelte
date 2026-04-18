<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { ApiError, get, post } from "$lib/api";
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
        GitBranch,
    } from "lucide-svelte";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import YamlEditorTab from "./components/YamlEditorTab.svelte";
    import MetaTab from "./components/MetaTab.svelte";

    import HistoryTab from "./components/HistoryTab.svelte";
    import GraphEditorTab from "./components/GraphEditorTab.svelte";

    let dataflowName = $derived(page.params.id as string);
    let dataflow = $state<any>(null);
    let mediaStatus = $state<any>(null);
    let loading = $state(true);
    let isRunConflictDialogOpen = $state(false);
    let activeTab = $state("graph");
    type RunFailureNotice = {
        summary: string;
        details: string[];
        nextStep: string;
        toast: string;
        raw: string;
    };
    let lastRunError = $state<RunFailureNotice | null>(null);

    function parseRunFailure(error: unknown): RunFailureNotice {
        const message =
            error instanceof ApiError
                ? error.rawMessage || error.message
                : error instanceof Error
                  ? error.message
                  : String(error ?? "Run failed");
        const lines = message
            .split("\n")
            .map((line) => line.trim())
            .filter(Boolean);

        const cleaned: string[] = [];
        let skippingLocation = false;

        for (const line of lines) {
            if (line.startsWith("dataflow start triggered:")) continue;
            if (line === "[ERROR]" || line === "Caused by:") continue;
            if (line === "Location:") {
                skippingLocation = true;
                continue;
            }
            if (skippingLocation) {
                skippingLocation = false;
                continue;
            }
            cleaned.push(line.replace(/^\d+:\s*/, ""));
        }

        const deduped = cleaned.filter(
            (line, index) => cleaned.indexOf(line) === index,
        );
        const flattened = deduped.join(" ").replace(/\s+/g, " ").trim();

        if (
            /no such file or directory/i.test(flattened) ||
            /os error 2/i.test(flattened)
        ) {
            const pathMatch = flattened.match(/(\/[^\s,)]+)/);
            const pathText = pathMatch ? ` at ${pathMatch[1]}` : "";
            return {
                summary: `Dora Manager could not start one of the configured node commands${pathText} because the file was not found.`,
                details: deduped,
                nextStep:
                    "Check the node path or reinstall the missing node, then save the workspace and run it again.",
                toast: "Run failed: a node path could not be found.",
                raw: message,
            };
        }

        if (/permission denied/i.test(flattened)) {
            return {
                summary:
                    "Dora Manager could not start one of the configured node commands because the process was not allowed to execute it.",
                details: deduped,
                nextStep:
                    "Check the node file permissions or command path, then save the workspace and run it again.",
                toast: "Run failed: a node command is not executable.",
                raw: message,
            };
        }

        return {
            summary: "Dora Manager could not start this workspace.",
            details: deduped,
            nextStep:
                "Review the workspace configuration, fix the failing node or command, then run it again.",
            toast: "Run failed: the workspace could not be started.",
            raw: message,
        };
    }

    function missingNodeImportEntries(executable: any) {
        const entries = executable?.missing_nodes_with_git_url;
        if (!entries) return [];
        return Object.entries(entries) as [string, string][];
    }

    async function loadDataflow() {
        loading = true;
        try {
            const [res, status] = await Promise.all([
                get(`/dataflows/${dataflowName}`),
                get("/media/status").catch(() => null),
            ]);
            dataflow = res;
            mediaStatus = status;
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
        if (
            dataflow?.executable?.requires_media_backend &&
            mediaStatus?.status !== "ready"
        ) {
            toast.error(
                "This dataflow requires dm-server media support. Open Settings > Media first.",
            );
            return;
        }

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
            lastRunError = null;
            const res: any = await post("/runs/start", {
                name: dataflowName,
                yaml: dataflow.yaml,
                force,
                view_json: dataflow.view ? JSON.stringify(dataflow.view) : undefined,
            });
            toast.success("Started dataflow");
            if (res.run_id) {
                goto(`/runs/${res.run_id}`);
            }
        } catch (e: any) {
            lastRunError = parseRunFailure(e);
            toast.error(lastRunError.toast);
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
                {#if dataflow?.executable?.requires_media_backend}
                    <Badge
                        variant="outline"
                        class={mediaStatus?.status === "ready"
                            ? "bg-sky-50 text-sky-700 border-sky-200"
                            : "bg-amber-50 text-amber-800 border-amber-200"}
                    >
                        {mediaStatus?.status === "ready"
                            ? "Media Ready"
                            : "Media Required"}
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
                    disabled={!dataflow?.executable?.can_run ||
                        (dataflow?.executable?.requires_media_backend &&
                            mediaStatus?.status !== "ready")}
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
        {#if dataflow?.executable?.invalid_yaml}
            <div
                class="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-900"
            >
                <div class="flex items-start gap-2">
                    <AlertCircle class="size-4 mt-0.5 shrink-0" />
                    <div class="space-y-1">
                        <p class="font-medium">This workspace has invalid YAML</p>
                        <p>
                            Fix the syntax in <span class="font-mono">dataflow.yml</span>
                            before running it again.
                        </p>
                        {#if dataflow.executable.error}
                            <p class="font-mono break-words text-red-800/90">
                                {dataflow.executable.error}
                            </p>
                        {/if}
                    </div>
                </div>
            </div>
        {:else if dataflow?.executable?.missing_nodes?.length}
            <div
                class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950"
            >
                <div class="flex items-start gap-2">
                    <AlertTriangle class="size-4 mt-0.5 shrink-0" />
                    <div class="space-y-2">
                        <p class="font-medium">
                            This workspace is missing {dataflow.executable.missing_node_count}
                            node{dataflow.executable.missing_node_count === 1 ? "" : "s"}
                        </p>
                        <p>
                            Install or restore the missing node{dataflow.executable.missing_node_count === 1 ? "" : "s"},
                            then save the workspace again.
                        </p>
                        <div class="flex flex-wrap gap-2">
                            {#each dataflow.executable.missing_nodes as nodeId}
                                <span
                                    class="rounded-full border border-amber-300 bg-white/70 px-2 py-1 font-mono text-xs"
                                >
                                    {nodeId}
                                </span>
                            {/each}
                        </div>
                        {#if missingNodeImportEntries(dataflow.executable).length}
                            <div class="space-y-1">
                                <p class="font-medium">Known import sources</p>
                                {#each missingNodeImportEntries(dataflow.executable) as [nodeId, gitUrl]}
                                    <p class="font-mono break-all text-xs text-amber-900/90">
                                        {nodeId}: dm node import {gitUrl}
                                    </p>
                                {/each}
                            </div>
                        {/if}
                    </div>
                </div>
            </div>
        {/if}

        {#if dataflow?.executable?.requires_media_backend && mediaStatus?.status !== "ready"}
            <div
                class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900"
            >
                This dataflow uses media-capable nodes:
                <span class="font-mono"
                    >{dataflow.executable.media_nodes?.join(", ")}</span
                >.
                Configure MediaMTX in <a class="font-medium underline" href="/settings"
                    >Settings</a
                > and restart `dm-server` before running it.
            </div>
        {/if}

        {#if lastRunError}
            <div
                class="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-900"
            >
                <div class="flex items-start gap-2">
                    <AlertTriangle class="size-4 mt-0.5 shrink-0" />
                    <div class="space-y-1">
                        <p class="font-medium">Last run attempt failed</p>
                        <p class="break-words">{lastRunError.summary}</p>
                        {#if lastRunError.details.length}
                            <div class="space-y-1 pt-1">
                                {#each lastRunError.details as detail}
                                    <p class="break-words text-red-800/90">{detail}</p>
                                {/each}
                            </div>
                        {/if}
                        <p class="text-red-700/80">
                            {lastRunError.nextStep}
                        </p>
                        <details class="pt-1 text-xs text-red-700/80">
                            <summary class="cursor-pointer select-none">
                                Show raw technical detail
                            </summary>
                            <pre class="mt-2 whitespace-pre-wrap break-words font-mono text-[11px] leading-5">{lastRunError.raw}</pre>
                        </details>
                    </div>
                </div>
            </div>
        {/if}

        <!-- Workspace Main Content -->
        <Tabs.Root value="graph" onValueChange={(v) => { activeTab = v; }} class="flex-1 flex flex-col min-h-0">
            <Tabs.List class="w-fit mb-4">
                <Tabs.Trigger value="graph" class="gap-2">
                    <GitBranch class="size-4" />
                    Graph Editor
                </Tabs.Trigger>
                <Tabs.Trigger value="yaml" class="gap-2">
                    <Code class="size-4" />
                    dataflow.yml
                </Tabs.Trigger>
                <Tabs.Trigger value="meta" class="gap-2">
                    <FileText class="size-4" />
                    flow.json
                </Tabs.Trigger>

                <Tabs.Trigger value="history" class="gap-2">
                    <History class="size-4" />
                    History
                </Tabs.Trigger>
            </Tabs.List>

            <Tabs.Content
                value="graph"
                class="flex-1 flex flex-col min-h-0 overflow-hidden mt-0"
            >
                {#if activeTab === "graph" && dataflow?.yaml !== undefined}
                    <GraphEditorTab
                        {dataflowName}
                        yamlStr={dataflow.yaml || ""}
                        viewJson={dataflow.view || {}}
                    />
                {/if}
            </Tabs.Content>

            <Tabs.Content
                value="yaml"
                class="flex-1 flex flex-col min-h-0 overflow-hidden mt-0"
            >
                {#if activeTab === "yaml" && dataflow?.yaml !== undefined}
                    <YamlEditorTab
                        {dataflowName}
                        initialYaml={dataflow.yaml || ""}
                        onCodeUpdated={(newYaml, refreshedDataflow) => {
                            if (refreshedDataflow) {
                                dataflow = refreshedDataflow;
                            } else if (dataflow) {
                                dataflow.yaml = newYaml;
                            }
                            lastRunError = null;
                        }}
                    />
                {/if}
            </Tabs.Content>



            <Tabs.Content
                value="meta"
                class="flex-1 border rounded-md bg-card shadow-sm flex flex-col min-h-0 overflow-hidden mt-0"
            >
                {#if activeTab === "meta" && dataflow?.meta !== undefined}
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
                {#if activeTab === "history"}
                    <HistoryTab {dataflowName} onRestored={loadDataflow} />
                {/if}
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
