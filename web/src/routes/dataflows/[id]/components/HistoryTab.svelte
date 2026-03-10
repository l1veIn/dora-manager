<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import { toast } from "svelte-sonner";
    import { Button } from "$lib/components/ui/button/index.js";
    import {
        History,
        RefreshCw,
        Clock,
        FileCode,
        RotateCcw,
    } from "lucide-svelte";
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import CodeMirror from "svelte-codemirror-editor";
    import { yaml } from "@codemirror/lang-yaml";
    import { mode } from "mode-watcher";
    import { oneDark } from "@codemirror/theme-one-dark";

    let { dataflowName, onRestored } = $props<{
        dataflowName: string;
        onRestored?: () => void;
    }>();

    let historyList = $state<any[]>([]);
    let loading = $state(true);

    let selectedVersion = $state<string | null>(null);
    let selectedYaml = $state<string>("");
    let loadingVersion = $state(false);
    let isRestoring = $state(false);

    async function loadHistory() {
        loading = true;
        try {
            const res: any = await get(`/dataflows/${dataflowName}/history`);
            historyList = res || [];
        } catch (e: any) {
            // A 404 indicates the history feature hasn't captured any snapshots yet
            if (e.message.includes("not found")) {
                historyList = [];
            } else {
                toast.error(`Failed to load history: ${e.message}`);
            }
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadHistory();
    });

    async function viewVersion(version: string) {
        selectedVersion = version;
        loadingVersion = true;
        try {
            const res: any = await get(
                `/dataflows/${dataflowName}/history/${version}`,
            );
            selectedYaml = res.yaml || "";
        } catch (e: any) {
            toast.error(`Failed to load version: ${e.message}`);
            selectedVersion = null;
        } finally {
            loadingVersion = false;
        }
    }

    async function restoreVersion() {
        if (!selectedVersion) return;
        isRestoring = true;
        try {
            await post(
                `/dataflows/${dataflowName}/history/${selectedVersion}/restore`,
            );
            toast.success("Version restored successfully");
            selectedVersion = null;
            onRestored?.();
            await loadHistory();
        } catch (e: any) {
            toast.error(`Restore failed: ${e.message}`);
        } finally {
            isRestoring = false;
        }
    }

    function formatTime(isoStr: string) {
        if (!isoStr) return "Unknown";
        return new Date(isoStr).toLocaleString();
    }
</script>

<div class="flex flex-col h-full w-full">
    {#if loading}
        <div class="flex-1 flex justify-center items-center">
            <RefreshCw class="size-6 animate-spin opacity-50" />
        </div>
    {:else if historyList.length === 0}
        <div
            class="flex-1 flex flex-col items-center justify-center p-12 text-center border-2 border-dashed rounded-lg bg-muted/10 mx-6 mt-6"
        >
            <History class="size-12 text-muted-foreground mb-4 opacity-50" />
            <h3 class="text-lg font-medium">No History Available</h3>
            <p class="text-sm text-muted-foreground mt-1 max-w-sm">
                This dataflow has not been saved or modified yet. Revisions will
                appear here automatically.
            </p>
        </div>
    {:else}
        <div class="flex-1 overflow-auto p-6">
            <div class="border rounded-md bg-card shadow-sm overflow-hidden">
                <table class="w-full text-sm text-left">
                    <thead class="bg-muted/50 text-muted-foreground">
                        <tr>
                            <th class="px-4 py-3 font-medium">Version ID</th>
                            <th class="px-4 py-3 font-medium">Modified At</th>
                            <th class="px-4 py-3 font-medium">Size</th>
                            <th class="px-4 py-3 font-medium text-right"
                                >Actions</th
                            >
                        </tr>
                    </thead>
                    <tbody class="divide-y">
                        {#each historyList as entry}
                            <tr class="hover:bg-muted/20 transition-colors">
                                <td
                                    class="px-4 py-3 font-mono text-xs text-primary"
                                    >{entry.version}</td
                                >
                                <td class="px-4 py-3">
                                    <div
                                        class="flex items-center gap-2 text-muted-foreground"
                                    >
                                        <Clock class="size-3" />
                                        {formatTime(entry.modified_at)}
                                    </div>
                                </td>
                                <td class="px-4 py-3 text-muted-foreground">
                                    {Math.round(entry.size / 1024)} KB
                                </td>
                                <td class="px-4 py-3 text-right">
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onclick={() =>
                                            viewVersion(entry.version)}
                                    >
                                        <FileCode class="size-3 mr-2" />
                                        View & Restore
                                    </Button>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        </div>
    {/if}

    <Dialog.Root
        open={!!selectedVersion && !isRestoring}
        onOpenChange={(v) => {
            if (!v) selectedVersion = null;
        }}
    >
        <Dialog.Content class="max-w-4xl max-h-[85vh] flex flex-col">
            <Dialog.Header>
                <Dialog.Title
                    >Version Details: <span
                        class="font-mono text-primary text-sm ml-2"
                        >{selectedVersion}</span
                    ></Dialog.Title
                >
                <Dialog.Description>
                    Review the configuration of this past revision before
                    restoring.
                </Dialog.Description>
            </Dialog.Header>

            <div
                class="flex-1 min-h-[400px] border rounded-md overflow-scroll relative my-4"
            >
                {#if loadingVersion}
                    <div
                        class="absolute inset-0 flex justify-center items-center bg-card"
                    >
                        <RefreshCw class="size-6 animate-spin opacity-50" />
                    </div>
                {:else}
                    <div
                        class="absolute inset-0 [&_.cm-editor]:h-full [&_.cm-scroller]:font-mono [&_.cm-scroller]:text-sm"
                    >
                        <CodeMirror
                            value={selectedYaml}
                            lang={yaml()}
                            readonly={true}
                            theme={mode && mode.current === "dark"
                                ? oneDark
                                : undefined}
                            styles={{
                                "&": {
                                    height: "100%",
                                    width: "100%",
                                    backgroundColor: "transparent",
                                    color: "inherit",
                                },
                                ".cm-gutters": {
                                    backgroundColor: "transparent",
                                    borderRight: "1px solid hsl(var(--border))",
                                },
                                "&.cm-focused": {
                                    outline: "none",
                                },
                            }}
                        />
                    </div>
                {/if}
            </div>

            <Dialog.Footer>
                <Button
                    variant="outline"
                    onclick={() => (selectedVersion = null)}>Cancel</Button
                >
                <Button
                    disabled={loadingVersion || isRestoring}
                    onclick={restoreVersion}
                >
                    <RotateCcw class="size-4 mr-2" />
                    {isRestoring ? "Restoring..." : "Restore Revision"}
                </Button>
            </Dialog.Footer>
        </Dialog.Content>
    </Dialog.Root>
</div>
