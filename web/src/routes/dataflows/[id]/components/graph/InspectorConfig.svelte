<script lang="ts">
    import { onMount, untrack, type Snippet } from 'svelte';
    import { get, post } from '$lib/api';
    import { RefreshCw, Settings, X, Globe, FileCode2, Info } from 'lucide-svelte';
    import { Label } from '$lib/components/ui/label/index.js';
    import { Input } from '$lib/components/ui/input/index.js';
    import { Textarea } from '$lib/components/ui/textarea/index.js';
    import { Switch } from '$lib/components/ui/switch/index.js';
    import { Slider } from '$lib/components/ui/slider/index.js';
    import { Button } from '$lib/components/ui/button/index.js';
    import * as Select from '$lib/components/ui/select/index.js';
    import * as RadioGroup from '$lib/components/ui/radio-group/index.js';
    import { PathPicker } from '$lib/components/ui/path-picker/index.js';
    import { toast } from 'svelte-sonner';

    let {
        node,
        onUpdateConfig,
        dataflowName
    }: {
        node: any;
        onUpdateConfig: (newConfig: any) => void;
        dataflowName: string;
    } = $props();

    let loading = $state(true);
    let aggregatedFields = $state<Record<string, any>>({});
    
    // Tracks local edits keyed by field name to quickly update the UI before pushing
    let formData = $state<Record<string, any>>({});

    let currentNodeId = $state('');

    async function loadConfig() {
        if (!node || !dataflowName) return;
        loading = true;
        try {
            const apiRes: any = await get(`/dataflows/${dataflowName}/config-schema`);
            let aggregatedNode = apiRes.nodes?.find((n: any) => n.yaml_id === node.id);
            if (aggregatedNode) {
                aggregatedFields = aggregatedNode.fields || {};
            } else {
                aggregatedFields = {};
            }
            
            const initialForm: Record<string, any> = {};
            for (const [key, field] of Object.entries(aggregatedFields)) {
                initialForm[key] = field.effective_value ?? field.schema?.default ?? null;
            }
            formData = initialForm;
        } catch (e) {
            console.error("Failed to load aggregated config schema", e);
            aggregatedFields = {};
        } finally {
            loading = false;
        }
    }

    $effect(() => {
        if (node && node.id !== currentNodeId) {
            currentNodeId = node.id;
            untrack(() => {
                loadConfig();
            });
        }
    });

    async function handleFieldChange(key: string, value: any) {
        formData[key] = value;
        const field = aggregatedFields[key];
        const isSecret = field?.schema?.secret === true;

        if (isSecret) {
            // Write to Global Node config
            try {
                // Fetch the current global config from API to merge the change
                const currentGlobal: any = await get(`/nodes/${node.data.nodeType}/config`);
                const newGlobal = { ...(currentGlobal || {}), [key]: value };
                await post(`/nodes/${node.data.nodeType}/config`, newGlobal);
                
                // Reload config to update labels (e.g. from default -> global)
                loadConfig();
            } catch (e: any) {
                toast.error(`Failed to save global config: ${e.message}`);
            }
        } else {
            // Write to Inline YAML config
            // We read the existing `node.data.config`, merge this single key, and push up
            const currentInlineConfig = { ...(node.data.config || {}) };
            currentInlineConfig[key] = value;
            onUpdateConfig(currentInlineConfig);
            // Updating the YAML might take some time to propagate via inspector. Reloading ensures tag updates.
            loadConfig();
        }
    }

    async function handleReset(key: string) {
        const field = aggregatedFields[key];
        const isSecret = field?.schema?.secret === true;

        if (isSecret) {
            try {
                const currentGlobal: any = await get(`/nodes/${node.data.nodeType}/config`);
                if (currentGlobal && key in currentGlobal) {
                    delete currentGlobal[key];
                    await post(`/nodes/${node.data.nodeType}/config`, currentGlobal);
                }
            } catch (e: any) {
                 toast.error(`Reset global config failed: ${e.message}`);
            }
        } else {
            const currentInlineConfig = { ...(node.data.config || {}) };
            if (key in currentInlineConfig) {
                delete currentInlineConfig[key];
                onUpdateConfig(currentInlineConfig);
            }
        }
        loadConfig();
    }
</script>

{#if loading}
    <div class="flex justify-center p-8">
        <RefreshCw class="size-5 animate-spin opacity-50" />
    </div>
{:else if Object.keys(aggregatedFields).length === 0}
    <div class="flex flex-col items-center justify-center p-6 text-center border border-dashed rounded-md bg-muted/10">
        <Settings class="size-8 text-muted-foreground mb-3 opacity-30" />
        <p class="text-xs text-muted-foreground">This node exposes no configuration.</p>
    </div>
{:else}
    <div class="flex flex-col space-y-5 pb-8">
        {#each Object.entries(aggregatedFields) as [key, aggField]}
            {@const s = aggField.schema}
            {@const isSecret = s?.secret === true}
            {@const source = aggField.effective_source}
            {@const isDefault = source === 'default' || source === 'unset'}
            <div class="flex flex-col space-y-1.5">
                <div class="flex items-center justify-between">
                    <Label
                        for="{node.id}-{key}"
                        class="font-medium text-xs flex items-center gap-2"
                    >
                        {key}
                        
                        {#if source === 'inline'}
                           <span class="text-[9px] text-blue-600 bg-blue-100 dark:text-blue-400 dark:bg-blue-900/30 px-1.5 py-0.5 rounded-sm flex items-center gap-1"><FileCode2 class="size-2.5"/> inline</span>
                        {:else if source === 'node'}
                           <span class="text-[9px] text-purple-600 bg-purple-100 dark:text-purple-400 dark:bg-purple-900/30 px-1.5 py-0.5 rounded-sm flex items-center gap-1"><Globe class="size-2.5"/> global</span>
                        {:else}
                           <span class="text-[9px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded-sm flex items-center gap-1"><Info class="size-2.5"/> default</span>
                        {/if}

                        {#if s?.env}
                            <span class="text-[9px] text-muted-foreground font-mono bg-muted px-1.5 py-0.5 rounded-sm">
                                {s?.env}
                            </span>
                        {/if}
                    </Label>

                    {#if !isDefault}
                        <Button variant="ghost" size="icon" class="h-5 w-5 text-muted-foreground hover:text-destructive shrink-0" onclick={() => handleReset(key)} title="Reset to default">
                            <X class="size-3"/>
                        </Button>
                    {/if}
                </div>

                <!-- Widgets -->
                {#if s?.["x-widget"]?.type === "select"}
                    <Select.Root
                        type="single"
                        value={String(formData[key] ?? "")}
                        onValueChange={(v) => {
                            const isNum = typeof s?.["x-widget"].options?.[0] === "number";
                            handleFieldChange(key, isNum ? Number(v) : v);
                        }}
                    >
                        <Select.Trigger class="w-full text-xs h-8 {isDefault ? 'opacity-80' : ''}">
                            {formData[key] || "Select..."}
                        </Select.Trigger>
                        <Select.Content class="z-[100]">
                            {#each s?.["x-widget"].options as opt}
                                <Select.Item value={String(opt)} class="text-xs">{opt}</Select.Item>
                            {/each}
                        </Select.Content>
                    </Select.Root>
                {:else if s?.["x-widget"]?.type === "slider"}
                    <div class="flex items-center gap-3 mt-1 {isDefault ? 'opacity-80' : ''}">
                        <Slider
                            type="single"
                            value={formData[key] ?? 0}
                            min={s?.["x-widget"].min ?? 0}
                            max={s?.["x-widget"].max ?? 100}
                            step={s?.["x-widget"].step ?? 1}
                            onValueChange={(v: number) => handleFieldChange(key, v)}
                            class="flex-1"
                        />
                        <Input
                            type="number"
                            value={formData[key] ?? 0}
                            onchange={(e: Event) => handleFieldChange(key, Number((e.currentTarget as HTMLInputElement).value))}
                            class="w-14 h-7 text-xs px-2"
                        />
                    </div>
                {:else if s?.["x-widget"]?.type === "switch"}
                    <div class="flex items-center space-x-2 border rounded-md p-2 bg-muted/10 mt-1 {isDefault ? 'opacity-80' : ''}">
                        <Switch
                            id="{node.id}-{key}"
                            checked={formData[key] ?? false}
                            onCheckedChange={(v) => handleFieldChange(key, v)}
                            class="scale-75 origin-left"
                        />
                        <Label for="{node.id}-{key}" class="font-normal cursor-pointer flex-1 text-xs">
                            Enable
                        </Label>
                    </div>
                {:else if s?.["x-widget"]?.type === "radio"}
                    <RadioGroup.Root
                        value={String(formData[key] ?? "")}
                        onValueChange={(v) => {
                            const isNumber = typeof s?.["x-widget"].options?.[0] === "number";
                            handleFieldChange(key, isNumber ? Number(v) : v);
                        }}
                        class="flex flex-col space-y-1.5 mt-1 {isDefault ? 'opacity-80' : ''}"
                    >
                        {#each s?.["x-widget"].options as opt}
                            <div class="flex items-center space-x-2">
                                <RadioGroup.Item value={String(opt)} id="{node.id}-{key}-{opt}" class="size-3.5" />
                                <Label for="{node.id}-{key}-{opt}" class="text-xs font-normal">{opt}</Label>
                            </div>
                        {/each}
                    </RadioGroup.Root>
                {:else if s?.["x-widget"]?.type === "checkbox"}
                    <div class="flex flex-wrap gap-x-3 gap-y-2 mt-1 {isDefault ? 'opacity-80' : ''}">
                        {#each s?.["x-widget"].options as opt}
                            <div class="flex items-center space-x-1.5">
                                <input
                                    type="checkbox"
                                    id="{node.id}-{key}-{opt}"
                                    checked={(formData[key] || []).includes(String(opt))}
                                    onchange={(e: Event) => {
                                        const checked = (e.currentTarget as HTMLInputElement).checked;
                                        const current = Array.isArray(formData[key]) ? [...formData[key]] : [];
                                        const val = String(opt);
                                        if (checked && !current.includes(val)) current.push(val);
                                        else if (!checked) {
                                            const idx = current.indexOf(val);
                                            if (idx >= 0) current.splice(idx, 1);
                                        }
                                        handleFieldChange(key, current);
                                    }}
                                    class="size-3.5 rounded border-input"
                                />
                                <Label for="{node.id}-{key}-{opt}" class="text-xs font-normal">{opt}</Label>
                            </div>
                        {/each}
                    </div>
                {:else if s?.["x-widget"]?.type === "file"}
                    <div class="{isDefault ? 'opacity-80' : ''}">
                        <PathPicker
                            mode="file"
                            id="{node.id}-{key}"
                            bind:value={formData[key]}
                            onValueChange={() => handleFieldChange(key, formData[key])}
                        />
                    </div>
                {:else if s?.["x-widget"]?.type === "directory"}
                    <div class="{isDefault ? 'opacity-80' : ''}">
                        <PathPicker
                            mode="directory"
                            id="{node.id}-{key}"
                            bind:value={formData[key]}
                            onValueChange={() => handleFieldChange(key, formData[key])}
                        />
                    </div>
                {:else if s?.type === "string"}
                    <Input
                        id="{node.id}-{key}"
                        type={isSecret ? "password" : "text"}
                        bind:value={formData[key]}
                        onchange={() => handleFieldChange(key, formData[key])}
                        placeholder={isDefault ? (s?.default || "") : ""}
                        class="h-8 text-xs {isDefault ? 'opacity-80 italic' : ''}"
                    />
                {:else if s?.type === "number"}
                    <Input
                        id="{node.id}-{key}"
                        type="number"
                        bind:value={formData[key]}
                        onchange={() => handleFieldChange(key, formData[key])}
                        placeholder={isDefault ? String(s?.default || "") : ""}
                        class="h-8 text-xs {isDefault ? 'opacity-80' : ''}"
                    />
                {:else if s?.type === "boolean"}
                    <div class="flex items-center space-x-2 border rounded-md p-2 bg-muted/10 mt-1 {isDefault ? 'opacity-80' : ''}">
                        <input
                            type="checkbox"
                            id="{node.id}-{key}"
                            checked={formData[key] ?? false}
                            onchange={(e: Event) => handleFieldChange(key, (e.currentTarget as HTMLInputElement).checked)}
                            class="size-3.5 rounded border-input"
                        />
                        <Label for="{node.id}-{key}" class="font-normal cursor-pointer flex-1 text-xs">
                            Enable
                        </Label>
                    </div>
                {:else}
                    <Textarea
                        id="{node.id}-{key}"
                        value={typeof formData[key] === "object" ? JSON.stringify(formData[key], null, 2) : String(formData[key] ?? "")}
                        onchange={(e: Event) => {
                            let val: any;
                            try { val = JSON.parse((e.currentTarget as HTMLTextAreaElement).value); } 
                            catch { val = (e.currentTarget as HTMLTextAreaElement).value; }
                            handleFieldChange(key, val);
                        }}
                        class="font-mono text-xs min-h-[80px] {isDefault ? 'opacity-80' : ''}"
                    />
                {/if}
                
                {#if s?.description}
                    <p class="text-[10px] text-muted-foreground leading-snug pt-1">
                        {s.description}
                        {#if isSecret && source === 'inline'}
                            <span class="text-amber-500 font-semibold inline-block pt-0.5 mt-0.5 border-t border-amber-500/20 block">
                                Warning: Secret is currently saved inline in the flow. Please write it into the node global config for security.
                            </span>
                        {:else if isSecret && isDefault}
                            <span class="text-amber-500/80 inline-block pt-0.5 mt-0.5 border-t border-amber-500/20 block">
                                Note: This sensitive value will be saved globally when edited.
                            </span>
                        {/if}
                    </p>
                {/if}
            </div>
        {/each}
    </div>
{/if}
