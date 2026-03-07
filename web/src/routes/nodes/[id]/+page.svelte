<script lang="ts">
    import { page } from "$app/state";
    import { onMount, tick } from "svelte";
    import { get, post, getText } from "$lib/api";
    import { goto } from "$app/navigation";
    import { toast } from "svelte-sonner";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { Skeleton } from "$lib/components/ui/skeleton/index.js";
    import {
        RefreshCw,
        ArrowLeft,
        Trash2,
        Play,
        Save,
        Folder,
        File,
        FileText,
        Terminal,
        Box,
        Search,
        BookOpen,
        Settings,
    } from "lucide-svelte";

    // Markdown
    import { marked } from "marked";
    import DOMPurify from "dompurify";

    import CodeTab from "./components/CodeTab.svelte";
    import SettingsTab from "./components/SettingsTab.svelte";

    // Build the highlight component logic here to avoid huge dependencies if possible
    // We can just use standard pre formatting for now until required

    let nodeId = $derived(page.params.id);
    let node = $state<any>(null);
    let loading = $state(true);

    // File Browser state
    let files = $state<string[]>([]);
    let loadingFiles = $state(false);
    let selectedFile = $state<string | null>(null);
    let selectedFileContent = $state<string>("");
    let loadingFileContent = $state(false);
    let fileSearch = $state("");

    // Config state
    let configSchema = $state<any>(null);
    let originalConfig = $state<any>({});
    let formData = $state<Record<string, any>>({});
    let loadingConfig = $state(false);
    let savingConfig = $state(false);

    // Actions state
    let operation = $state<string | null>(null);

    $effect(() => {
        if (nodeId) {
            loadNodeDetails();
        }
    });

    async function loadNodeDetails() {
        loading = true;
        try {
            const status: any = await get(`/nodes/${nodeId}`);
            if (!status) {
                toast.error(`Node ${nodeId} not found`);
                goto("/nodes");
                return;
            }
            node = status;
            configSchema = status?.config_schema || null;

            // Load all other details in parallel
            await Promise.all([loadFiles(), loadConfig()]);
        } catch (e) {
            console.error("Failed to load node:", e);
            toast.error("Failed to load node details");
        } finally {
            loading = false;
        }
    }

    function parseMarkdown(md: string) {
        if (!md || md === "Loading...") {
            return md;
        }
        try {
            return DOMPurify.sanitize(marked.parse(md) as string);
        } catch (e) {
            return `<pre class="whitespace-pre-wrap">${md}</pre>`;
        }
    }

    async function loadFiles() {
        loadingFiles = true;
        try {
            const res: any = await get(`/nodes/${nodeId}/files`);
            if (Array.isArray(res)) {
                files = res.sort();
            }
        } catch (e) {
            console.error("Failed to load files:", e);
        } finally {
            loadingFiles = false;
        }
    }

    async function handleFileSelect(file: string) {
        selectedFile = file;
        loadingFileContent = true;
        selectedFileContent = "";

        try {
            // Because paths can contain slashes, we fetch as text.
            const content = await getText(`/nodes/${nodeId}/files/${file}`);
            selectedFileContent = content;
        } catch (e: any) {
            selectedFileContent = `Error loading file:\n${e.message || "Unknown error"}`;
        } finally {
            loadingFileContent = false;
        }
    }

    async function loadConfig() {
        loadingConfig = true;
        try {
            const cfg = await get(`/nodes/${nodeId}/config`);
            originalConfig = cfg || {};
            formData = { ...originalConfig };

            if (configSchema) {
                Object.keys(configSchema).forEach((key) => {
                    if (
                        formData[key] === undefined &&
                        configSchema[key].default !== undefined
                    ) {
                        formData[key] = configSchema[key].default;
                    }
                });
            }
        } catch (e) {
            console.error("Failed to load config", e);
        } finally {
            loadingConfig = false;
        }
    }

    async function saveConfig() {
        savingConfig = true;
        try {
            await fetch(`http://127.0.0.1:3210/api/nodes/${nodeId}/config`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(formData),
            });
            toast.success("Configuration saved");
            originalConfig = { ...formData };
        } catch (e: any) {
            toast.error(`Failed to save config: ${e.message}`);
        } finally {
            savingConfig = false;
        }
    }

    async function handleInstall() {
        operation = "installing";
        try {
            await post("/nodes/install", { id: nodeId });
            toast.success(`${nodeId} installed successfully`);
            await loadNodeDetails();
        } catch (e: any) {
            toast.error(`Failed to install: ${e.message}`);
        } finally {
            operation = null;
        }
    }

    async function handleUninstall() {
        if (!confirm(`Are you sure you want to delete ${nodeId}?`)) return;
        operation = "uninstalling";
        try {
            await post("/nodes/uninstall", { id: nodeId });
            toast.success(`${nodeId} uninstalled`);
            goto("/nodes");
        } catch (e: any) {
            toast.error(`Failed to uninstall: ${e.message}`);
            operation = null;
        }
    }

    let hasChanges = $derived(
        JSON.stringify(formData) !== JSON.stringify(originalConfig),
    );
    let needsInstall = $derived(
        !node?.executable || node.executable.trim() === "",
    );

    let filteredFiles = $derived(
        files.filter((f) => f.toLowerCase().includes(fileSearch.toLowerCase())),
    );
</script>

<div
    class="flex flex-col h-full pt-6 pb-6 px-4 md:px-8 w-full max-w-7xl mx-auto space-y-6 min-h-0"
>
    <!-- Breadcrumb and Actions Header -->
    <div class="flex items-start justify-between">
        <div class="flex flex-col gap-1">
            <Button
                variant="ghost"
                size="sm"
                class="w-fit -ml-2 text-muted-foreground hover:text-foreground mb-2"
                href="/nodes"
            >
                <ArrowLeft class="size-4 mr-1" />
                Back to Nodes
            </Button>
            <div class="flex items-center gap-3">
                <h1
                    class="text-3xl font-bold font-mono tracking-tight flex items-center gap-2"
                >
                    <Box class="size-6 text-primary" />
                    {nodeId}
                </h1>
                {#if node}
                    <Badge variant="outline" class="font-mono text-xs">
                        {node.version || "v0.0.0"}
                    </Badge>
                    <Badge variant="secondary" class="text-xs">
                        {node.language || "Unknown"}
                    </Badge>
                {/if}
            </div>
            {#if node?.description}
                <p class="text-muted-foreground mt-1 max-w-2xl text-sm">
                    {node.description}
                </p>
            {/if}
        </div>

        {#if node}
            <div class="flex items-center gap-2">
                {#if needsInstall}
                    <Button
                        disabled={operation === "installing"}
                        onclick={handleInstall}
                    >
                        {#if operation === "installing"}
                            <RefreshCw class="size-4 animate-spin mr-2" />
                        {:else}
                            <Play class="size-4 mr-2" />
                        {/if}
                        Install Node
                    </Button>
                {:else}
                    <Button
                        variant="outline"
                        disabled={operation === "installing"}
                        onclick={handleInstall}
                        title="Re-install this node (e.g. after code changes)"
                    >
                        {#if operation === "installing"}
                            <RefreshCw class="size-4 animate-spin mr-2" />
                        {:else}
                            <RefreshCw class="size-4 mr-2" />
                        {/if}
                        Re-install
                    </Button>
                {/if}
                <Button
                    variant="destructive"
                    class="gap-2"
                    disabled={operation === "uninstalling"}
                    onclick={handleUninstall}
                >
                    {#if operation === "uninstalling"}
                        <RefreshCw class="size-4 animate-spin" />
                    {:else}
                        <Trash2 class="size-4" />
                    {/if}
                    Delete
                </Button>
            </div>
        {/if}
    </div>

    {#if loading}
        <div class="space-y-4">
            <Skeleton class="h-10 w-full max-w-md" />
            <Skeleton class="h-[60vh] w-full rounded-lg" />
        </div>
    {:else}
        <!-- Main Content Area -->
        <Tabs.Root value="code" class="flex-1 flex flex-col min-h-0">
            <Tabs.List class="w-fit mb-4">
                <Tabs.Trigger value="code" class="gap-2">
                    <Terminal class="size-4" />
                    Code
                </Tabs.Trigger>
                <Tabs.Trigger value="config" class="gap-2">
                    <Settings class="size-4" />
                    Settings
                </Tabs.Trigger>
            </Tabs.List>

            <!-- CODE TAB -->
            <Tabs.Content
                value="code"
                class="flex-1 flex flex-col min-h-0 overflow-hidden"
            >
                <CodeTab
                    {files}
                    {loadingFiles}
                    {selectedFile}
                    {selectedFileContent}
                    {loadingFileContent}
                    onSelectFile={handleFileSelect}
                    parsedMarkdown={parseMarkdown}
                />
            </Tabs.Content>

            <!-- CONFIG TAB -->
            <Tabs.Content
                value="config"
                class="flex-1 border rounded-md bg-card flex flex-col min-h-0 overflow-hidden"
            >
                <SettingsTab
                    {configSchema}
                    {originalConfig}
                    bind:formData
                    {loadingConfig}
                    {savingConfig}
                    onSave={saveConfig}
                    onRevert={() => (formData = { ...originalConfig })}
                />
            </Tabs.Content>
        </Tabs.Root>
    {/if}
</div>
