<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import { toast } from "svelte-sonner";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import { Slider } from "$lib/components/ui/slider/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
    import { Save, RefreshCw, Settings } from "lucide-svelte";
    import { PathPicker } from "$lib/components/ui/path-picker/index.js";

    let { dataflowName } = $props<{
        dataflowName: string;
    }>();

    let loading = $state(true);
    let isSaving = $state(false);

    // Core state
    let configurableNodes = $state<any[]>([]);
    let nodeSchemas = $state<Record<string, any>>({});

    // configData[yaml_id] = { param1: ..., param2: ... }
    let configData = $state<Record<string, any>>({});
    let originalConfigStr = $state("");

    let hasChanges = $derived(JSON.stringify(configData) !== originalConfigStr);

    async function loadConfigData() {
        loading = true;
        try {
            const schemaRes: any = await get(
                `/dataflows/${dataflowName}/config-schema`,
            );

            // The new API returns { executable: {...}, nodes: [...] }
            const nodes = schemaRes.nodes || [];

            // Only keep configurable and resolved nodes for the UI
            configurableNodes = nodes.filter(
                (n: any) => n.configurable && n.resolved,
            );

            const schemas: Record<string, any> = {};
            const formData: Record<string, any> = {};

            for (const node of configurableNodes) {
                // Reconstruct the schema format expected by the UI from the new 'fields' structure
                const nodeSchema: Record<string, any> = {};
                const nodeFormData: Record<string, any> = {};

                const fields = node.fields || {};
                for (const [key, fieldData] of Object.entries(fields)) {
                    const f: any = fieldData;
                    nodeSchema[key] = f.schema || {};

                    // Pre-fill form data with inline overrides if they exist
                    if (f.inline_value !== undefined) {
                        nodeFormData[key] = f.inline_value;
                    }
                }

                schemas[node.node_id] = nodeSchema;
                formData[node.yaml_id] = nodeFormData;
            }

            nodeSchemas = schemas;
            configData = formData;
            originalConfigStr = JSON.stringify(formData);
        } catch (e: any) {
            toast.error(`Failed to load configuration schema: ${e.message}`);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadConfigData();
    });

    async function saveConfig() {
        if (!dataflowName || !hasChanges) return;
        isSaving = true;
        try {
            await post(`/dataflows/${dataflowName}/config`, {
                config: configData,
            });
            originalConfigStr = JSON.stringify(configData);
            toast.success("Configuration overrides saved successfully");
        } catch (e: any) {
            toast.error(`Save failed: ${e.message}`);
        } finally {
            isSaving = false;
        }
    }

    function revertConfig() {
        configData = JSON.parse(originalConfigStr);
    }
</script>

<div class="flex flex-col h-full w-full">
    {#if loading}
        <div class="flex-1 flex justify-center items-center">
            <RefreshCw class="size-6 animate-spin opacity-50" />
        </div>
    {:else if configurableNodes.length === 0}
        <div
            class="flex-1 flex flex-col items-center justify-center p-12 text-center border-2 border-dashed rounded-lg bg-muted/10 mx-6 mt-6"
        >
            <Settings class="size-12 text-muted-foreground mb-4 opacity-50" />
            <h3 class="text-lg font-medium">No Configurable Nodes</h3>
            <p class="text-sm text-muted-foreground mt-1 max-w-sm">
                The nodes used in this dataflow do not expose any configuration
                parameters.
            </p>
        </div>
    {:else}
        <div class="flex-1 overflow-auto p-6">
            <div class="w-full grid grid-cols-1 gap-6 pb-2">
                {#each configurableNodes as node}
                    {@const schema = nodeSchemas[node.node_id] || {}}
                    <div
                        class="flex flex-col space-y-3 p-5 border rounded-xl bg-card shadow-sm justify-start"
                    >
                        <!-- Header -->
                        <div class="flex flex-col gap-1 pb-3 border-b">
                            <div class="flex items-center gap-2">
                                <Settings
                                    class="size-4 text-primary shrink-0"
                                />
                                <h3
                                    class="font-mono font-bold text-base truncate"
                                    title={node.yaml_id}
                                >
                                    {node.yaml_id}
                                </h3>
                            </div>
                            <span
                                class="text-[10px] text-muted-foreground font-mono truncate"
                                title={node.node_id}>{node.node_id}</span
                            >
                        </div>

                        <!-- Fields -->
                        <div class="flex-1">
                            {#if Object.keys(schema).length === 0}
                                <div
                                    class="text-sm text-muted-foreground italic mt-2"
                                >
                                    No schema definitions found.
                                </div>
                            {:else}
                                <div class="flex flex-col space-y-6">
                                    {#each Object.entries(schema) as [key, schemaDef]}
                                        {@const s = schemaDef as any}
                                        <div class="flex flex-col space-y-2">
                                            <Label
                                                for="{node.yaml_id}-{key}"
                                                class="font-medium text-sm flex items-center gap-2"
                                            >
                                                {key}
                                                {#if s?.env}
                                                    <span
                                                        class="text-[10px] text-muted-foreground font-mono bg-muted px-1.5 py-0.5 rounded-sm"
                                                    >
                                                        {s?.env}
                                                    </span>
                                                {/if}
                                            </Label>

                                            <!-- X-Widget Renderers -->
                                            {#if s?.["x-widget"]?.type === "select"}
                                                <Select.Root
                                                    type="single"
                                                    bind:value={
                                                        configData[
                                                            node.yaml_id
                                                        ][key]
                                                    }
                                                >
                                                    <Select.Trigger
                                                        class="w-full"
                                                    >
                                                        {configData[
                                                            node.yaml_id
                                                        ][key] ??
                                                            s?.default ??
                                                            "Select..."}
                                                    </Select.Trigger>
                                                    <Select.Content>
                                                        {#each s?.["x-widget"].options as opt}
                                                            <Select.Item
                                                                value={opt}
                                                                >{opt}</Select.Item
                                                            >
                                                        {/each}
                                                    </Select.Content>
                                                </Select.Root>
                                            {:else if s?.["x-widget"]?.type === "slider"}
                                                <div
                                                    class="flex items-center gap-4"
                                                >
                                                    <Slider
                                                        type="single"
                                                        value={configData[
                                                            node.yaml_id
                                                        ][key] ||
                                                            s?.default ||
                                                            0}
                                                        min={s?.["x-widget"]
                                                            .min || 0}
                                                        max={s?.["x-widget"]
                                                            .max || 100}
                                                        step={s?.["x-widget"]
                                                            .step || 1}
                                                        onValueChange={(
                                                            v: number,
                                                        ) =>
                                                            (configData[
                                                                node.yaml_id
                                                            ][key] = v)}
                                                        class="flex-1"
                                                    />
                                                    <Input
                                                        type="number"
                                                        bind:value={
                                                            configData[
                                                                node.yaml_id
                                                            ][key]
                                                        }
                                                        class="w-16 h-8 text-xs px-2"
                                                    />
                                                </div>
                                            {:else if s?.["x-widget"]?.type === "switch"}
                                                <div
                                                    class="flex items-center space-x-2 border rounded-md p-2 bg-muted/20"
                                                >
                                                    <Switch
                                                        id="{node.yaml_id}-{key}"
                                                        checked={!!configData[
                                                            node.yaml_id
                                                        ][key]}
                                                        onCheckedChange={(v) =>
                                                            (configData[
                                                                node.yaml_id
                                                            ][key] = v)}
                                                    />
                                                    <Label
                                                        for="{node.yaml_id}-{key}"
                                                        class="font-normal cursor-pointer flex-1 text-sm"
                                                    >
                                                        Enable
                                                    </Label>
                                                </div>
                                            {:else if s?.["x-widget"]?.type === "radio"}
                                                <RadioGroup.Root
                                                    value={String(
                                                        configData[
                                                            node.yaml_id
                                                        ][key],
                                                    )}
                                                    onValueChange={(v) => {
                                                        const isNumber =
                                                            typeof s?.[
                                                                "x-widget"
                                                            ].options[0] ===
                                                            "number";
                                                        configData[
                                                            node.yaml_id
                                                        ][key] = isNumber
                                                            ? Number(v)
                                                            : v;
                                                    }}
                                                    class="flex flex-col space-y-2 mt-1"
                                                >
                                                    {#each s?.["x-widget"].options as opt}
                                                        <div
                                                            class="flex items-center space-x-2"
                                                        >
                                                            <RadioGroup.Item
                                                                value={String(
                                                                    opt,
                                                                )}
                                                                id="{node.yaml_id}-{key}-{opt}"
                                                            />
                                                            <Label
                                                                for="{node.yaml_id}-{key}-{opt}"
                                                                class="text-sm font-normal"
                                                                >{opt}</Label
                                                            >
                                                        </div>
                                                    {/each}
                                                </RadioGroup.Root>
                                            {:else if s?.["x-widget"]?.type === "checkbox"}
                                                <div class="flex flex-wrap gap-x-4 gap-y-2 mt-1">
                                                    {#each s?.["x-widget"].options as opt}
                                                        <div
                                                            class="flex items-center space-x-2"
                                                        >
                                                            <input
                                                                type="checkbox"
                                                                id="{node.yaml_id}-{key}-{opt}"
                                                                checked={(configData[node.yaml_id][key] || []).includes(String(opt))}
                                                                onchange={(e: Event) => {
                                                                    const checked = (e.currentTarget as HTMLInputElement).checked;
                                                                    const current = Array.isArray(configData[node.yaml_id][key]) ? [...configData[node.yaml_id][key]] : [];
                                                                    const val = String(opt);
                                                                    if (checked && !current.includes(val)) {
                                                                        current.push(val);
                                                                    } else if (!checked) {
                                                                        const idx = current.indexOf(val);
                                                                        if (idx >= 0) current.splice(idx, 1);
                                                                    }
                                                                    configData[node.yaml_id][key] = current;
                                                                }}
                                                                class="size-4 rounded"
                                                            />
                                                            <Label
                                                                for="{node.yaml_id}-{key}-{opt}"
                                                                class="text-sm font-normal"
                                                                >{opt}</Label
                                                            >
                                                        </div>
                                                    {/each}
                                                </div>
                                            {:else if s?.["x-widget"]?.type === "file"}
                                                <PathPicker
                                                    mode="file"
                                                    id="{node.yaml_id}-{key}"
                                                    bind:value={
                                                        configData[
                                                            node.yaml_id
                                                        ][key]
                                                    }
                                                    placeholder={s?.default || undefined}
                                                />
                                            {:else if s?.["x-widget"]?.type === "directory"}
                                                <PathPicker
                                                    mode="directory"
                                                    id="{node.yaml_id}-{key}"
                                                    bind:value={
                                                        configData[
                                                            node.yaml_id
                                                        ][key]
                                                    }
                                                    placeholder={s?.default || undefined}
                                                />
                                            {:else if s?.type === "string"}
                                                <Input
                                                    id="{node.yaml_id}-{key}"
                                                    bind:value={
                                                        configData[
                                                            node.yaml_id
                                                        ][key]
                                                    }
                                                    placeholder={s?.default ||
                                                        ""}
                                                />
                                            {:else if s?.type === "number"}
                                                <Input
                                                    id="{node.yaml_id}-{key}"
                                                    type="number"
                                                    bind:value={
                                                        configData[
                                                            node.yaml_id
                                                        ][key]
                                                    }
                                                    placeholder={s?.default ||
                                                        ""}
                                                />
                                            {:else if s?.type === "boolean"}
                                                <div
                                                    class="flex items-center space-x-2 border rounded-md p-3"
                                                >
                                                    <input
                                                        type="checkbox"
                                                        id="{node.yaml_id}-{key}"
                                                        checked={configData[
                                                            node.yaml_id
                                                        ][key]}
                                                        onchange={(e: Event) =>
                                                            (configData[
                                                                node.yaml_id
                                                            ][key] = (
                                                                e.currentTarget as HTMLInputElement
                                                            ).checked)}
                                                        class="size-4"
                                                    />
                                                    <Label
                                                        for="{node.yaml_id}-{key}"
                                                        class="font-normal cursor-pointer flex-1"
                                                    >
                                                        Enable {key}
                                                    </Label>
                                                </div>
                                            {:else}
                                                <Textarea
                                                    id="{node.yaml_id}-{key}"
                                                    value={typeof configData[
                                                        node.yaml_id
                                                    ][key] === "object"
                                                        ? JSON.stringify(
                                                              configData[
                                                                  node.yaml_id
                                                              ][key],
                                                              null,
                                                              2,
                                                          )
                                                        : String(
                                                              configData[
                                                                  node.yaml_id
                                                              ][key] ?? "",
                                                          )}
                                                    onchange={(e: Event) => {
                                                        try {
                                                            configData[
                                                                node.yaml_id
                                                            ][key] = JSON.parse(
                                                                (
                                                                    e.currentTarget as HTMLTextAreaElement
                                                                ).value,
                                                            );
                                                        } catch (err) {
                                                            configData[
                                                                node.yaml_id
                                                            ][key] = (
                                                                e.currentTarget as HTMLTextAreaElement
                                                            ).value;
                                                        }
                                                    }}
                                                    class="font-mono text-xs min-h-[100px]"
                                                />
                                            {/if}
                                            {#if s?.description}
                                                <p
                                                    class="text-xs text-muted-foreground"
                                                >
                                                    {s.description}
                                                </p>
                                            {/if}
                                        </div>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                    </div>
                {/each}
            </div>
        </div>

        <!-- Fixed footer similar to SettingsTab -->
        <div
            class="p-4 px-6 border-t flex justify-end gap-3 bg-muted/30 shrink-0"
        >
            <Button
                variant="outline"
                onclick={revertConfig}
                disabled={!hasChanges || isSaving}
            >
                Revert Changes
            </Button>
            <Button disabled={!hasChanges || isSaving} onclick={saveConfig}>
                <Save class="size-4 mr-2" />
                {isSaving ? "Saving..." : "Save Overrides"}
            </Button>
        </div>
    {/if}
</div>
