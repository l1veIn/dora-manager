<script lang="ts">
    import {
        Save,
        RefreshCw,
        Settings,
        FileText,
        FolderOpen,
    } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import { Slider } from "$lib/components/ui/slider/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import * as RadioGroup from "$lib/components/ui/radio-group/index.js";

    let {
        configSchema,
        originalConfig,
        formData = $bindable(),
        loadingConfig = false,
        savingConfig = false,
        onSave = () => {},
        onRevert = () => {},
    } = $props<{
        configSchema: any;
        originalConfig: any;
        formData: any;
        loadingConfig?: boolean;
        savingConfig?: boolean;
        onSave?: () => void;
        onRevert?: () => void;
    }>();

    let hasChanges = $derived(
        JSON.stringify(formData) !== JSON.stringify(originalConfig),
    );
</script>

{#if loadingConfig}
    <div class="flex justify-center p-12">
        <RefreshCw class="size-6 animate-spin opacity-50" />
    </div>
{:else if !configSchema}
    <div
        class="flex-1 flex flex-col items-center justify-center border-2 border-dashed rounded-lg p-12 bg-muted/10"
    >
        <Settings class="size-12 text-muted-foreground mb-4 opacity-20" />
        <h3 class="text-lg font-medium">No Configuration</h3>
        <p class="text-muted-foreground mt-1">
            This node does not expose any configuration parameters.
        </p>
    </div>
{:else}
    <div class="flex flex-col h-full w-full">
        <div class="flex-1 overflow-auto p-6">
            <div class="w-full grid grid-cols-1 gap-6 pb-2">
                {#each Object.entries(configSchema) as [key, schema]}
                    {@const s = schema as any}
                    <div
                        class="flex flex-col space-y-3 p-5 border rounded-xl bg-card shadow-sm justify-start"
                    >
                        <Label
                            for={key}
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

                        {#if s?.["x-widget"]?.type === "select"}
                            <Select.Root
                                type="single"
                                bind:value={formData[key]}
                            >
                                <Select.Trigger class="w-full">
                                    {formData[key] ?? s?.default ?? "Select..."}
                                </Select.Trigger>
                                <Select.Content>
                                    {#each s?.["x-widget"].options as opt}
                                        <Select.Item value={opt}
                                            >{opt}</Select.Item
                                        >
                                    {/each}
                                </Select.Content>
                            </Select.Root>
                        {:else if s?.["x-widget"]?.type === "slider"}
                            <div class="flex items-center gap-4 mt-2">
                                <Slider
                                    type="single"
                                    value={formData[key] || s?.default || 0}
                                    min={s?.["x-widget"].min || 0}
                                    max={s?.["x-widget"].max || 100}
                                    step={s?.["x-widget"].step || 1}
                                    onValueChange={(v: number) =>
                                        (formData[key] = v)}
                                    class="flex-1"
                                />
                                <Input
                                    type="number"
                                    bind:value={formData[key]}
                                    class="w-20"
                                />
                            </div>
                        {:else if s?.["x-widget"]?.type === "switch"}
                            <div
                                class="flex items-center space-x-2 border rounded-md p-3"
                            >
                                <Switch
                                    id={key}
                                    checked={!!formData[key]}
                                    onCheckedChange={(v) => (formData[key] = v)}
                                />
                                <Label
                                    for={key}
                                    class="font-normal cursor-pointer flex-1"
                                >
                                    Enable {key}
                                </Label>
                            </div>
                        {:else if s?.["x-widget"]?.type === "radio"}
                            <RadioGroup.Root
                                value={String(formData[key])}
                                onValueChange={(v) => {
                                    // Convert back to number if options are numbers
                                    const isNumber =
                                        typeof s?.["x-widget"].options[0] ===
                                        "number";
                                    formData[key] = isNumber ? Number(v) : v;
                                }}
                                class="flex flex-col space-y-1 mt-2"
                            >
                                {#each s?.["x-widget"].options as opt}
                                    <div class="flex items-center space-x-2">
                                        <RadioGroup.Item
                                            value={String(opt)}
                                            id={`${key}-${opt}`}
                                        />
                                        <Label for={`${key}-${opt}`}
                                            >{opt}</Label
                                        >
                                    </div>
                                {/each}
                            </RadioGroup.Root>
                        {:else if s?.["x-widget"]?.type === "file"}
                            <div class="flex items-center gap-2">
                                <FileText
                                    class="size-4 text-muted-foreground shrink-0"
                                />
                                <Input
                                    id={key}
                                    bind:value={formData[key]}
                                    placeholder={s?.default ||
                                        "Enter file path..."}
                                    class="flex-1 font-mono text-xs"
                                />
                            </div>
                        {:else if s?.["x-widget"]?.type === "directory"}
                            <div class="flex items-center gap-2">
                                <FolderOpen
                                    class="size-4 text-muted-foreground shrink-0"
                                />
                                <Input
                                    id={key}
                                    bind:value={formData[key]}
                                    placeholder={s?.default ||
                                        "Enter directory path..."}
                                    class="flex-1 font-mono text-xs"
                                />
                            </div>
                        {:else if s?.type === "string"}
                            <Input
                                id={key}
                                bind:value={formData[key]}
                                placeholder={s?.default || ""}
                            />
                        {:else if s?.type === "number"}
                            <Input
                                id={key}
                                type="number"
                                bind:value={formData[key]}
                                placeholder={s?.default || ""}
                            />
                        {:else if s?.type === "boolean"}
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
                            <Textarea
                                id={key}
                                value={typeof formData[key] === "object"
                                    ? JSON.stringify(formData[key], null, 2)
                                    : String(formData[key] ?? "")}
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
                                class="font-mono text-xs min-h-[100px]"
                            />
                        {/if}
                        {#if s?.description}
                            <p class="text-xs text-muted-foreground">
                                {s.description}
                            </p>
                        {/if}
                    </div>
                {/each}
            </div>
        </div>

        <div class="p-4 px-6 border-t flex justify-end gap-3 bg-muted/30">
            <Button
                variant="outline"
                onclick={onRevert}
                disabled={!hasChanges || savingConfig}
            >
                Revert Changes
            </Button>
            <Button onclick={onSave} disabled={!hasChanges || savingConfig}>
                {#if savingConfig}
                    <RefreshCw class="size-4 animate-spin mr-2" />
                {:else}
                    <Save class="size-4 mr-2" />
                {/if}
                Save Configuration
            </Button>
        </div>
    </div>
{/if}
