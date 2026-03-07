<script lang="ts">
    import { post } from "$lib/api";
    import { toast } from "svelte-sonner";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Save } from "lucide-svelte";

    let {
        dataflowName,
        meta = {},
        onMetaUpdated,
    } = $props<{
        dataflowName: string;
        meta?: any;
        onMetaUpdated?: (newMeta: any) => void;
    }>();

    let internalMeta = $state<any>({});
    let tagsString = $state("");
    let isSaving = $state(false);

    // Deep copy helper since we are modifying fields
    $effect(() => {
        if (meta && meta.updated_at !== internalMeta.updated_at) {
            internalMeta = JSON.parse(JSON.stringify(meta));
            tagsString = (meta.tags || []).join(", ");
        }
    });

    async function saveMeta() {
        if (!dataflowName) return;
        isSaving = true;
        try {
            const newTags = tagsString
                .split(",")
                .map((t: string) => t.trim())
                .filter(Boolean);
            const payload = { ...internalMeta, tags: newTags };

            await post(`/dataflows/${dataflowName}/meta`, payload);
            toast.success("Metadata saved successfully");
            onMetaUpdated?.(payload);
        } catch (e: any) {
            toast.error(`Failed to save metadata: ${e.message}`);
        } finally {
            isSaving = false;
        }
    }
</script>

<div class="flex flex-col h-full w-full">
    <div class="flex-1 overflow-auto p-6">
        <div class="max-w-4xl mx-auto space-y-6">
            <div>
                <h3 class="text-lg font-medium">Dataflow Metadata</h3>
                <p class="text-sm text-muted-foreground">
                    Manage the business properties and labels of this workflow.
                </p>
            </div>

            <div class="w-full grid grid-cols-1 gap-6 pb-2">
                <div
                    class="flex flex-col space-y-3 p-5 border rounded-xl bg-card shadow-sm justify-start"
                >
                    <Label for="name" class="font-medium text-sm"
                        >Display Name</Label
                    >
                    <Input
                        id="name"
                        placeholder="E.g. Production Data Ingestion"
                        bind:value={internalMeta.name}
                    />
                    <p class="text-[10px] text-muted-foreground mt-auto pt-2">
                        The descriptive name of the dataflow shown in the UI.
                    </p>
                </div>

                <div
                    class="flex flex-col space-y-3 p-5 border rounded-xl bg-card shadow-sm justify-start"
                >
                    <Label for="description" class="font-medium text-sm"
                        >Description</Label
                    >
                    <Textarea
                        id="description"
                        placeholder="Describe what this dataflow does..."
                        rows={4}
                        bind:value={internalMeta.description}
                        class="resize-none"
                    />
                </div>

                <div
                    class="flex flex-col space-y-3 p-5 border rounded-xl bg-card shadow-sm justify-start"
                >
                    <Label for="tags" class="font-medium text-sm">Tags</Label>
                    <Input
                        id="tags"
                        placeholder="e.g. production, etl, computer-vision"
                        bind:value={tagsString}
                    />
                    <p class="text-[10px] text-muted-foreground mt-auto pt-2">
                        Comma-separated tags for organization and search.
                    </p>
                </div>
            </div>
        </div>
    </div>

    <!-- Footer -->
    <div class="p-4 px-6 border-t flex justify-end gap-3 bg-muted/30 shrink-0">
        <Button disabled={isSaving} onclick={saveMeta}>
            <Save class="mr-2 size-4" />
            {isSaving ? "Saving..." : "Save Metadata"}
        </Button>
    </div>
</div>
