<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import RunStatusBadge from "./RunStatusBadge.svelte";
    import { StopCircle, FileText, Loader2, ChevronLeft } from "lucide-svelte";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { goto } from "$app/navigation";
    import { get, getText } from "$lib/api";
    import CodeMirror from "svelte-codemirror-editor";
    import { yaml } from "@codemirror/lang-yaml";
    import { mode } from "mode-watcher";
    import { oneDark } from "@codemirror/theme-one-dark";

    let {
        run,
        onStop = () => {},
        isStopping = false,
    } = $props<{
        run: any;
        onStop?: () => void;
        isStopping?: boolean;
    }>();

    let isYamlOpen = $state(false);
    let isTranspiledOpen = $state(false);
    let yamlContent = $state("");
    let transpiledContent = $state("");
    let loadingYaml = $state(false);
    let loadingTranspiled = $state(false);

    async function openYaml() {
        if (!run?.id) return;
        isYamlOpen = true;
        if (!yamlContent) {
            loadingYaml = true;
            try {
                const res: string = await getText(`/runs/${run.id}/dataflow`);
                yamlContent = res;
            } catch (e: any) {
                yamlContent = `Error: ${e.message}`;
            } finally {
                loadingYaml = false;
            }
        }
    }

    async function openTranspiled() {
        if (!run?.id) return;
        isTranspiledOpen = true;
        if (!transpiledContent) {
            loadingTranspiled = true;
            try {
                const res: string = await getText(`/runs/${run.id}/transpiled`);
                transpiledContent = res;
            } catch (e: any) {
                transpiledContent = `Error: ${e.message}`;
            } finally {
                loadingTranspiled = false;
            }
        }
    }

    function formatTime(ts: string) {
        if (!ts) return "-";
        return new Date(ts).toLocaleString();
    }

    function calculateDuration(start: string, end: string) {
        if (!start) return "-";
        const t1 = new Date(start).getTime();
        const t2 = end ? new Date(end).getTime() : Date.now();
        const diffMs = t2 - t1;

        const secs = Math.floor(diffMs / 1000);
        if (secs < 60) return `${secs}s`;
        const mins = Math.floor(secs / 60);
        const remSecs = secs % 60;
        return `${mins}m ${remSecs}s`;
    }
</script>

<header
    class="flex items-center justify-between px-4 h-14 border-b bg-card/95 backdrop-blur-sm shrink-0 z-20 w-full relative"
>
    <div class="flex items-center gap-3">
        <Button
            variant="ghost"
            size="icon"
            onclick={() => goto("/runs")}
            class="shrink-0 h-8 w-8 text-muted-foreground mr-1"
        >
            <ChevronLeft class="size-4" />
        </Button>
        <span
            class="text-sm text-muted-foreground hidden sm:inline-flex font-medium select-none"
            >Runs <span class="mx-2 opacity-50">/</span></span
        >
        <h1
            class="text-sm font-bold tracking-tight font-mono text-foreground flex items-center"
        >
            {run?.name || "Loading..."}
        </h1>

        {#if run}
            <div class="ml-2 flex flex-row items-center gap-2">
                <RunStatusBadge status={run.status} />
                {#if run.has_panel}
                    <Badge
                        variant="outline"
                        class="font-mono text-[9px] uppercase px-1.5 py-0 shadow-sm bg-indigo-50 text-indigo-700 border-indigo-200 dark:bg-indigo-900/40 dark:text-indigo-400 dark:border-indigo-800"
                        >Panel Present</Badge
                    >
                {/if}
            </div>
        {/if}

        {#if run?.outcome_summary}
            <div
                class="ml-4 pl-4 border-l hidden lg:flex items-center text-xs text-muted-foreground max-w-sm truncate h-4"
                title={run.outcome_summary}
            >
                {run.outcome_summary}
            </div>
        {/if}
    </div>

    <div class="flex items-center gap-2">
        {#if run?.status === "running"}
            <Button
                variant="destructive"
                size="sm"
                onclick={onStop}
                class="h-8"
                disabled={isStopping}
            >
                {#if isStopping}
                    <Loader2 class="size-3.5 mr-1.5 animate-spin" />
                    Stopping...
                {:else}
                    <StopCircle class="size-3.5 mr-1.5" />
                    Stop
                {/if}
            </Button>
        {/if}
        {#if run?.id}
            <Button variant="outline" size="sm" onclick={openYaml} class="h-8">
                <FileText class="size-3.5 mr-1.5" />
                YAML
            </Button>
            <Button
                variant="outline"
                size="sm"
                onclick={openTranspiled}
                class="h-8"
            >
                <FileText class="size-3.5 mr-1.5" />
                Transpiled
            </Button>
        {/if}
    </div>
</header>

<!-- YAML Modal -->
<Dialog.Root bind:open={isYamlOpen}>
    <Dialog.Content class="sm:max-w-[700px] h-[80vh] flex flex-col">
        <Dialog.Header>
            <Dialog.Title>Dataflow Source YAML</Dialog.Title>
            <Dialog.Description
                >The original un-transpiled dataflow snippet</Dialog.Description
            >
        </Dialog.Header>
        <div
            class="flex-1 w-full border rounded-lg overflow-y-auto [&_.cm-editor]:h-full [&_.cm-scroller]:font-mono [&_.cm-scroller]:text-sm"
        >
            {#if loadingYaml}
                <div
                    class="h-full flex items-center justify-center text-muted-foreground"
                >
                    <Loader2 class="size-6 animate-spin mr-2" /> Loading...
                </div>
            {:else}
                <CodeMirror
                    value={yamlContent}
                    lang={yaml()}
                    readonly={true}
                    theme={mode && mode.current === "dark"
                        ? oneDark
                        : undefined}
                    styles={{
                        "&": {
                            height: "100%",
                            background: "transparent",
                            color: "inherit",
                        },
                        ".cm-gutters": {
                            background: "transparent",
                            color: "inherit",
                            borderRight: "1px solid var(--border)",
                        },
                    }}
                />
            {/if}
        </div>
        <Dialog.Footer>
            <Button variant="outline" onclick={() => (isYamlOpen = false)}
                >Close</Button
            >
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>

<!-- Transpiled Modal -->
<Dialog.Root bind:open={isTranspiledOpen}>
    <Dialog.Content class="sm:max-w-[700px] h-[80vh] flex flex-col">
        <Dialog.Header>
            <Dialog.Title>Transpiled Dataflow YAML</Dialog.Title>
            <Dialog.Description
                >The resolved configuration executing in the network</Dialog.Description
            >
        </Dialog.Header>
        <div
            class="flex-1 w-full border rounded-lg overflow-y-auto [&_.cm-editor]:h-full [&_.cm-scroller]:font-mono [&_.cm-scroller]:text-sm"
        >
            {#if loadingTranspiled}
                <div
                    class="h-full flex items-center justify-center text-muted-foreground"
                >
                    <Loader2 class="size-6 animate-spin mr-2" /> Loading...
                </div>
            {:else}
                <CodeMirror
                    value={transpiledContent}
                    lang={yaml()}
                    readonly={true}
                    theme={mode && mode.current === "dark"
                        ? oneDark
                        : undefined}
                    styles={{
                        "&": {
                            height: "100%",
                            background: "transparent",
                            color: "inherit",
                        },
                        ".cm-gutters": {
                            background: "transparent",
                            color: "inherit",
                            borderRight: "1px solid var(--border)",
                        },
                    }}
                />
            {/if}
        </div>
        <Dialog.Footer>
            <Button variant="outline" onclick={() => (isTranspiledOpen = false)}
                >Close</Button
            >
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
