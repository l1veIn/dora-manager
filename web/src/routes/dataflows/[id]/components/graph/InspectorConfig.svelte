<script lang="ts">
    import { onMount } from 'svelte';
    import { get } from '$lib/api';
    import { RefreshCw, Settings } from 'lucide-svelte';
    import { Label } from '$lib/components/ui/label/index.js';
    import { Input } from '$lib/components/ui/input/index.js';
    import { Textarea } from '$lib/components/ui/textarea/index.js';
    import { Switch } from '$lib/components/ui/switch/index.js';
    import { Slider } from '$lib/components/ui/slider/index.js';
    import * as Select from '$lib/components/ui/select/index.js';
    import * as RadioGroup from '$lib/components/ui/radio-group/index.js';
    import { PathPicker } from '$lib/components/ui/path-picker/index.js';

    import { untrack } from 'svelte';

    let {
        node,
        onUpdateConfig,
    }: {
        node: any;
        onUpdateConfig: (newConfig: any) => void;
    } = $props();

    let loading = $state(true);
    let schema = $state<Record<string, any> | null>(null);
    let formData = $state<Record<string, any>>({});
    
    // Keep track of the node id we are currently inspecting
    let currentNodeId = $state('');

    async function loadSchema() {
        if (!node) return;
        loading = true;
        try {
            const apiRes: any = await get(`/nodes/${node.data.nodeType}`);
            schema = apiRes?.config_schema || null;
            
            const existing = node.data.config || {};
            const initialForm = JSON.parse(JSON.stringify(existing));

            formData = initialForm;
        } catch (e) {
            console.error("Failed to load config schema", e);
            schema = null;
        } finally {
            loading = false;
        }
    }

    // Force reload when selected node changes
    $effect(() => {
        if (node && node.id !== currentNodeId) {
            currentNodeId = node.id;
            untrack(() => {
                loadSchema();
            });
        }
    });

    // Auto-sync formData changes back to the node config safely
    $effect(() => {
        // Read deeply to subscribe to user UI changes
        const currentDataStr = JSON.stringify(formData);
        // Track node config updates from the parent 
        const nodeConfigStr = JSON.stringify(node?.data?.config || {});
        
        if (!loading && schema) {
            untrack(() => {
                // Only dispatch if data legitimately diverges to break infinite loop
                if (currentDataStr !== nodeConfigStr && node?.id === currentNodeId) {
                    onUpdateConfig(JSON.parse(currentDataStr));
                }
            });
        }
    });

</script>

{#if loading}
    <div class="flex justify-center p-8">
        <RefreshCw class="size-5 animate-spin opacity-50" />
    </div>
{:else if !schema || Object.keys(schema).length === 0}
    <div class="flex flex-col items-center justify-center p-6 text-center border border-dashed rounded-md bg-muted/10">
        <Settings class="size-8 text-muted-foreground mb-3 opacity-30" />
        <p class="text-xs text-muted-foreground">This node exposes no configuration.</p>
    </div>
{:else}
    <div class="flex flex-col space-y-5 pb-8">
        {#each Object.entries(schema) as [key, schemaDef]}
            {@const s = schemaDef as any}
            <div class="flex flex-col space-y-1.5">
                <Label
                    for="{node.id}-{key}"
                    class="font-medium text-xs flex items-center gap-2"
                >
                    {key}
                    {#if s?.env}
                        <span class="text-[9px] text-muted-foreground font-mono bg-muted px-1.5 py-0.5 rounded-sm">
                            {s?.env}
                        </span>
                    {/if}
                </Label>

                <!-- Widgets -->
                {#if s?.["x-widget"]?.type === "select"}
                    <Select.Root
                        type="single"
                        value={String(formData[key] ?? s?.default ?? "")}
                        onValueChange={(v) => {
                            const isNum = typeof s?.["x-widget"].options?.[0] === "number";
                            formData[key] = isNum ? Number(v) : v;
                        }}
                    >
                        <Select.Trigger class="w-full text-xs h-8">
                            {formData[key] ?? s?.default ?? "Select..."}
                        </Select.Trigger>
                        <Select.Content class="z-[100]">
                            {#each s?.["x-widget"].options as opt}
                                <Select.Item value={String(opt)} class="text-xs">{opt}</Select.Item>
                            {/each}
                        </Select.Content>
                    </Select.Root>
                {:else if s?.["x-widget"]?.type === "slider"}
                    <div class="flex items-center gap-3 mt-1">
                        <Slider
                            type="single"
                            value={formData[key] ?? s?.default ?? 0}
                            min={s?.["x-widget"].min ?? 0}
                            max={s?.["x-widget"].max ?? 100}
                            step={s?.["x-widget"].step ?? 1}
                            onValueChange={(v: number) => (formData[key] = v)}
                            class="flex-1"
                        />
                        <Input
                            type="number"
                            value={formData[key] ?? s?.default ?? 0}
                            oninput={(e: Event) => (formData[key] = Number((e.currentTarget as HTMLInputElement).value))}
                            class="w-14 h-7 text-xs px-2"
                        />
                    </div>
                {:else if s?.["x-widget"]?.type === "switch"}
                    <div class="flex items-center space-x-2 border rounded-md p-2 bg-muted/10 mt-1">
                        <Switch
                            id="{node.id}-{key}"
                            checked={formData[key] ?? s?.default ?? false}
                            onCheckedChange={(v) => (formData[key] = v)}
                            class="scale-75 origin-left"
                        />
                        <Label for="{node.id}-{key}" class="font-normal cursor-pointer flex-1 text-xs">
                            Enable
                        </Label>
                    </div>
                {:else if s?.["x-widget"]?.type === "radio"}
                    <RadioGroup.Root
                        value={String(formData[key] ?? s?.default ?? "")}
                        onValueChange={(v) => {
                            const isNumber = typeof s?.["x-widget"].options?.[0] === "number";
                            formData[key] = isNumber ? Number(v) : v;
                        }}
                        class="flex flex-col space-y-1.5 mt-1"
                    >
                        {#each s?.["x-widget"].options as opt}
                            <div class="flex items-center space-x-2">
                                <RadioGroup.Item value={String(opt)} id="{node.id}-{key}-{opt}" class="size-3.5" />
                                <Label for="{node.id}-{key}-{opt}" class="text-xs font-normal">{opt}</Label>
                            </div>
                        {/each}
                    </RadioGroup.Root>
                {:else if s?.["x-widget"]?.type === "checkbox"}
                    <div class="flex flex-wrap gap-x-3 gap-y-2 mt-1">
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
                                        formData[key] = current;
                                    }}
                                    class="size-3.5 rounded border-input"
                                />
                                <Label for="{node.id}-{key}-{opt}" class="text-xs font-normal">{opt}</Label>
                            </div>
                        {/each}
                    </div>
                {:else if s?.["x-widget"]?.type === "file"}
                    <PathPicker
                        mode="file"
                        id="{node.id}-{key}"
                        bind:value={formData[key]}
                        placeholder={s?.default || undefined}
                    />
                {:else if s?.["x-widget"]?.type === "directory"}
                    <PathPicker
                        mode="directory"
                        id="{node.id}-{key}"
                        bind:value={formData[key]}
                        placeholder={s?.default || undefined}
                    />
                {:else if s?.type === "string"}
                    <Input
                        id="{node.id}-{key}"
                        bind:value={formData[key]}
                        placeholder={s?.default || ""}
                        class="h-8 text-xs"
                    />
                {:else if s?.type === "number"}
                    <Input
                        id="{node.id}-{key}"
                        type="number"
                        bind:value={formData[key]}
                        placeholder={s?.default || ""}
                        class="h-8 text-xs"
                    />
                {:else if s?.type === "boolean"}
                    <div class="flex items-center space-x-2 border rounded-md p-2 bg-muted/10 mt-1">
                        <input
                            type="checkbox"
                            id="{node.id}-{key}"
                            checked={formData[key] ?? s?.default ?? false}
                            onchange={(e: Event) => (formData[key] = (e.currentTarget as HTMLInputElement).checked)}
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
                            try { formData[key] = JSON.parse((e.currentTarget as HTMLTextAreaElement).value); } 
                            catch { formData[key] = (e.currentTarget as HTMLTextAreaElement).value; }
                        }}
                        class="font-mono text-xs min-h-[80px]"
                    />
                {/if}
                
                {#if s?.description}
                    <p class="text-[10px] text-muted-foreground leading-snug">
                        {s.description}
                    </p>
                {/if}
            </div>
        {/each}
    </div>
{/if}
