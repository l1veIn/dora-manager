<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { toast } from "svelte-sonner";
    import { get, getText } from "$lib/api";
    import {
        Save,
        RefreshCw,
        Trash2,
        Download,
        Play,
        BookOpen,
    } from "lucide-svelte";

    // Markdown parser
    import { marked } from "marked";
    import DOMPurify from "dompurify";

    let {
        open = $bindable(false),
        node = null,
        isRegistry = false,
        isInstalled = false,
        operation = null,
        onAction,
    } = $props<{
        open: boolean;
        node: any | null;
        isRegistry?: boolean;
        isInstalled?: boolean;
        operation?: string | null;
        onAction?: (action: string, id: string) => void;
    }>();

    let needsInstall = $derived(
        isInstalled && (!node?.executable || node.executable.trim() === ""),
    );

    let readmeContent = $state<string>("Loading...");
    let configSchema = $state<any>(null);
    let originalConfig = $state<any>({});
    let formData = $state<Record<string, any>>({});

    let loadingReadme = $state(false);
    let loadingConfig = $state(false);
    let savingConfig = $state(false);

    let parsedReadme = $derived(() => {
        if (
            !readmeContent ||
            readmeContent === "Loading..." ||
            readmeContent === "No README found for this node."
        ) {
            return readmeContent;
        }
        try {
            return DOMPurify.sanitize(marked.parse(readmeContent) as string);
        } catch (e) {
            return `<pre class="whitespace-pre-wrap">${readmeContent}</pre>`;
        }
    });

    $effect(() => {
        if (open && node) {
            loadReadme();
            if (!isRegistry || isInstalled) {
                loadConfig();
            }
        }
    });

    async function loadReadme() {
        loadingReadme = true;
        try {
            const res = await getText(`/nodes/${node.id}/readme`);
            readmeContent = res || "No README found for this node.";
        } catch (e) {
            readmeContent = "No README found for this node.";
        } finally {
            loadingReadme = false;
        }
    }

    async function loadConfig() {
        loadingConfig = true;
        try {
            const status: any = await get(`/nodes/${node.id}`);
            configSchema = status?.config_schema || null;

            const cfg = await get(`/nodes/${node.id}/config`);
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
            toast.error("Failed to load node configuration");
        } finally {
            loadingConfig = false;
        }
    }

    async function saveConfig() {
        savingConfig = true;
        try {
            await fetch(`http://127.0.0.1:3210/api/nodes/${node.id}/config`, {
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

    let hasChanges = $derived(
        JSON.stringify(formData) !== JSON.stringify(originalConfig),
    );
</script>

{#snippet buildConfigField(key: string, rawSchema: any)}
    {@const schema = rawSchema as {
        env?: string;
        type?: string;
        default?: any;
    }}
    <div class="space-y-2">
        <Label for={key} class="font-medium">
            {key}
            {#if schema?.env}
                <span class="text-xs text-muted-foreground ml-2 font-mono"
                    >({schema?.env})</span
                >
            {/if}
        </Label>

        {#if schema?.type === "string"}
            <Input
                id={key}
                bind:value={formData[key]}
                placeholder={schema?.default || ""}
            />
        {:else if schema?.type === "number"}
            <Input
                id={key}
                type="number"
                bind:value={formData[key]}
                placeholder={schema?.default || ""}
            />
        {:else if schema?.type === "boolean"}
            <div class="flex items-center space-x-2 border rounded-md p-3">
                <input
                    type="checkbox"
                    id={key}
                    checked={formData[key]}
                    onchange={(e: Event) =>
                        (formData[key] = (
                            e.currentTarget as HTMLInputElement
                        ).checked)}
                    class="size-4"
                />
                <Label for={key} class="font-normal cursor-pointer flex-1"
                    >Enable {key}</Label
                >
            </div>
        {:else}
            <!-- Fallback for objects/arrays -->
            <Textarea
                id={key}
                value={typeof formData[key] === "object"
                    ? JSON.stringify(formData[key], null, 2)
                    : String(formData[key] || "")}
                onchange={(e: Event) => {
                    try {
                        formData[key] = JSON.parse(
                            (e.currentTarget as HTMLTextAreaElement).value,
                        );
                    } catch (err) {
                        formData[key] = (
                            e.currentTarget as HTMLTextAreaElement
                        ).value;
                    }
                }}
                class="font-mono text-xs h-24"
            />
        {/if}
    </div>
{/snippet}

<Dialog.Root bind:open>
    <Dialog.Content
        class="w-[95vw] sm:max-w-3xl max-h-[70vh] flex flex-col pt-10"
    >
        {#if node}
            <div
                class="flex items-start justify-between absolute top-4 left-6 right-10"
            >
                <Dialog.Title
                    class="text-2xl font-mono flex items-center gap-2"
                >
                    {node.name || node.id}
                    <Badge
                        variant="secondary"
                        class="font-mono text-[10px] whitespace-nowrap"
                    >
                        {node.version || "v0.0.0"}
                    </Badge>
                </Dialog.Title>
            </div>

            <!-- Flex wrapper without scroll -->
            <div class="flex-1 flex flex-col min-h-0 mt-2 pb-4">
                <Tabs.Root
                    value="readme"
                    class="w-full flex-1 flex flex-col min-h-0 mt-4"
                >
                    {#if !isRegistry || isInstalled}
                        <Tabs.List class="mb-4 flex-shrink-0">
                            <Tabs.Trigger value="readme">README</Tabs.Trigger>
                            <Tabs.Trigger value="config"
                                >Configuration</Tabs.Trigger
                            >
                        </Tabs.List>
                    {/if}

                    <Tabs.Content
                        value="readme"
                        class="border rounded-md p-4 bg-muted/5 flex-1 overflow-y-auto min-h-[40vh]"
                    >
                        {#if loadingReadme}
                            <div
                                class="flex items-center justify-center h-48 opacity-50"
                            >
                                <RefreshCw class="size-6 animate-spin" />
                            </div>
                        {:else}
                            <div
                                class="prose prose-sm dark:prose-invert max-w-none prose-pre:bg-muted/50 prose-pre:border"
                            >
                                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                                {@html parsedReadme()}
                            </div>
                        {/if}
                    </Tabs.Content>

                    {#if !isRegistry || isInstalled}
                        <Tabs.Content
                            value="config"
                            class="flex-1 overflow-y-auto border rounded-md p-6 bg-muted/5 flex flex-col min-h-[40vh]"
                        >
                            {#if loadingConfig}
                                <div
                                    class="flex items-center justify-center h-48 opacity-50"
                                >
                                    <RefreshCw class="size-6 animate-spin" />
                                </div>
                            {:else if !configSchema}
                                <div
                                    class="flex items-center justify-center h-48 border rounded-md border-dashed bg-muted/10"
                                >
                                    <p class="text-muted-foreground">
                                        This node does not expose any
                                        configuration parameters.
                                    </p>
                                </div>
                            {:else}
                                <div class="space-y-6 flex-1">
                                    {#each Object.entries(configSchema) as [key, schema]}
                                        {@render buildConfigField(key, schema)}
                                    {/each}
                                </div>
                                <div
                                    class="mt-8 pt-4 border-t flex justify-end gap-3"
                                >
                                    <Button
                                        variant="outline"
                                        onclick={() =>
                                            (formData = { ...originalConfig })}
                                        disabled={!hasChanges || savingConfig}
                                    >
                                        Revert
                                    </Button>
                                    <Button
                                        onclick={saveConfig}
                                        disabled={!hasChanges || savingConfig}
                                    >
                                        {#if savingConfig}
                                            <RefreshCw
                                                class="size-4 animate-spin mr-2"
                                            />
                                        {:else}
                                            <Save class="size-4 mr-2" />
                                        {/if}
                                        Save Changes
                                    </Button>
                                </div>
                            {/if}
                        </Tabs.Content>
                    {/if}
                </Tabs.Root>
            </div>

            <!-- Footer for Node Actions -->
            {#if onAction}
                <Dialog.Footer class="mt-4 pt-4 border-t sm:justify-end">
                    {#if isRegistry}
                        {#if isInstalled && !needsInstall}
                            <Button variant="outline" disabled
                                >Installed ✓</Button
                            >
                        {:else if needsInstall}
                            <!-- <Button
                                disabled={operation === "installing"}
                                onclick={() => onAction("install", node.id)}
                            >
                                {#if operation === "installing"}
                                    <RefreshCw
                                        class="size-4 animate-spin mr-2"
                                    />
                                {:else}
                                    <Play class="size-4 mr-2" />
                                {/if}
                                Complete Install
                            </Button> -->
                        {:else}
                            <Button
                                disabled={operation === "downloading"}
                                onclick={() => onAction("download", node.id)}
                            >
                                {#if operation === "downloading"}
                                    <RefreshCw
                                        class="size-4 animate-spin mr-2"
                                    />
                                {:else}
                                    <Download class="size-4 mr-2" />
                                {/if}
                                Download
                            </Button>
                        {/if}
                    {:else}
                        <Button
                            variant="destructive"
                            class="gap-2"
                            disabled={operation === "uninstalling"}
                            onclick={() => onAction("uninstall", node.id)}
                        >
                            {#if operation === "uninstalling"}
                                <RefreshCw class="size-4 animate-spin" />
                            {:else}
                                <Trash2 class="size-4" />
                            {/if}
                            Delete
                        </Button>
                        <!-- Local Node -->
                        {#if needsInstall}
                            <Button
                                disabled={operation === "installing"}
                                onclick={() => onAction("install", node.id)}
                            >
                                {#if operation === "installing"}
                                    <RefreshCw
                                        class="size-4 animate-spin mr-2"
                                    />
                                {:else}
                                    <Play class="size-4 mr-2" />
                                {/if}
                                Install
                            </Button>
                        {:else}
                            <Button
                                variant="outline"
                                disabled={operation === "installing"}
                                onclick={() => onAction("install", node.id)}
                                title="Re-install this node (e.g. after code changes)"
                            >
                                {#if operation === "installing"}
                                    <RefreshCw
                                        class="size-4 animate-spin mr-2"
                                    />
                                {:else}
                                    <RefreshCw class="size-4 mr-2" />
                                {/if}
                                Re-install
                            </Button>
                        {/if}
                    {/if}
                </Dialog.Footer>
            {/if}
        {/if}
    </Dialog.Content>
</Dialog.Root>
