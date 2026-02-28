<script lang="ts">
    import * as Sheet from "$lib/components/ui/sheet/index.js";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import { toast } from "svelte-sonner";
    import { get, post } from "$lib/api";
    import { Save, RefreshCw } from "lucide-svelte";

    // Markdown parser (if available, otherwise fallback to basic formatting)
    import { marked } from "marked";
    import DOMPurify from "dompurify";

    let { open = $bindable(false), node = null } = $props<{
        open: boolean;
        node: any | null;
    }>();

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
            loadConfig();
        }
    });

    async function loadReadme() {
        loadingReadme = true;
        try {
            const res = await get(`/nodes/${node.id}/readme`);
            // Assuming the text is returned as is, or in a JSON structure
            readmeContent =
                typeof res === "string"
                    ? res
                    : (res as any)?.content || "No README found for this node.";
        } catch (e) {
            readmeContent = "No README found for this node.";
        } finally {
            loadingReadme = false;
        }
    }

    async function loadConfig() {
        loadingConfig = true;
        try {
            // Read schema from the node metadata
            const status: any = await get(`/nodes/${node.id}`);
            configSchema = status?.config_schema || null;

            // Fetch actual config values
            const cfg = await get(`/nodes/${node.id}/config`);
            originalConfig = cfg || {};

            // Initialize form data
            formData = { ...originalConfig };

            // Apply defaults from schema if not present
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
            // Need to change the JSON fetch to POST since we changed the backend to POST
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

    // Check if configuration has been modified
    let hasChanges = $derived(
        JSON.stringify(formData) !== JSON.stringify(originalConfig),
    );
</script>

<Sheet.Root bind:open>
    <Sheet.Content class="w-[90vw] sm:max-w-2xl overflow-y-auto">
        {#if node}
            <Sheet.Header class="mb-4">
                <Sheet.Title class="text-2xl font-mono"
                    >{node.name || node.id}</Sheet.Title
                >
                <Sheet.Description
                    >{node.description ||
                        "No description provided."}</Sheet.Description
                >
            </Sheet.Header>

            <Tabs.Root value="readme" class="w-full">
                <Tabs.List class="grid w-full grid-cols-2">
                    <Tabs.Trigger value="readme">README</Tabs.Trigger>
                    <Tabs.Trigger value="config">Configuration</Tabs.Trigger>
                </Tabs.List>

                <Tabs.Content
                    value="readme"
                    class="mt-4 border rounded-md p-4 min-h-[50vh]"
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

                <Tabs.Content
                    value="config"
                    class="mt-4 min-h-[50vh] flex flex-col"
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
                                This node does not expose any configuration
                                parameters.
                            </p>
                        </div>
                    {:else}
                        <div class="space-y-6 flex-1">
                            {#each Object.entries(configSchema) as [key, rawSchema]}
                                {@const schema = rawSchema as {
                                    env?: string;
                                    type?: string;
                                    default?: any;
                                }}
                                <div class="space-y-2">
                                    <Label for={key} class="font-medium">
                                        {key}
                                        {#if schema?.env}
                                            <span
                                                class="text-xs text-muted-foreground ml-2 font-mono"
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
                                        <div
                                            class="flex items-center space-x-2 border rounded-md p-3"
                                        >
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
                                            <Label
                                                for={key}
                                                class="font-normal cursor-pointer flex-1"
                                            >
                                                Enable {key}
                                            </Label>
                                        </div>
                                    {:else}
                                        <!-- Fallback for objects/arrays -->
                                        <Textarea
                                            id={key}
                                            value={typeof formData[key] ===
                                            "object"
                                                ? JSON.stringify(
                                                      formData[key],
                                                      null,
                                                      2,
                                                  )
                                                : String(formData[key] || "")}
                                            onchange={(e: Event) => {
                                                try {
                                                    formData[key] = JSON.parse(
                                                        (
                                                            e.currentTarget as HTMLTextAreaElement
                                                        ).value,
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
                            {/each}
                        </div>

                        <div
                            class="mt-8 pt-4 border-t flex justify-end gap-3 sticky bottom-0 bg-background py-2"
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
            </Tabs.Root>
        {/if}
    </Sheet.Content>
</Sheet.Root>
