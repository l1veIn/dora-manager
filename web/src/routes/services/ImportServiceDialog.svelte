<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { toast } from "svelte-sonner";
    import { post } from "$lib/api";
    import { RefreshCw, Download } from "lucide-svelte";

    let { open = $bindable(false), onImported } = $props<{
        open: boolean;
        onImported: () => void;
    }>();

    let source = $state("");
    let id = $state("");
    let importing = $state(false);

    // Validate ID (optional, but if provided should be valid)
    let isValidId = $derived(id.length === 0 || /^[a-z0-9-]+$/.test(id));
    let hasSource = $derived(source.trim().length > 0);

    async function handleImport() {
        if (id && !isValidId) {
            toast.error(
                "Service ID must contain only lowercase letters, numbers, and hyphens",
            );
            return;
        }
        if (!hasSource) {
            toast.error("Source is required");
            return;
        }

        importing = true;
        try {
            const payload: any = { source: source.trim() };
            if (id.trim()) {
                payload.id = id.trim();
            }

            await post("/services/import", payload);
            toast.success(`Service imported successfully!`);
            open = false; // Close dialog
            // Reset form
            source = "";
            id = "";
            // Notify parent
            onImported();
        } catch (e: any) {
            toast.error(`Failed to import service: ${e.message}`);
        } finally {
            importing = false;
        }
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2">
                <Download class="size-5 text-primary" />
                Import Service
            </Dialog.Title>
            <Dialog.Description>
                Import an existing Dora service from a local directory path or a
                Git repository URL.
            </Dialog.Description>
        </Dialog.Header>

        <div class="grid gap-4 py-4">
            <div class="grid gap-2">
                <Label for="service-source" class="font-medium"
                    >Source Path / URL <span class="text-red-500">*</span
                    ></Label
                >
                <Input
                    id="service-source"
                    bind:value={source}
                    placeholder="e.g., /path/to/my-service or https://github.com/..."
                    autocomplete="off"
                />
            </div>

            <div class="grid gap-2">
                <Label for="service-id" class="font-medium"
                    >Service ID Override (Optional)</Label
                >
                <Input
                    id="service-id"
                    bind:value={id}
                    placeholder="Leave empty to infer from source"
                    autocomplete="off"
                    class={id && !isValidId
                        ? "border-red-500 focus-visible:ring-red-500"
                        : ""}
                />
                {#if id && !isValidId}
                    <p class="text-xs text-red-500">
                        Only lowercase letters, numbers, and hyphens allowed.
                    </p>
                {/if}
            </div>
        </div>

        <Dialog.Footer>
            <Button
                variant="outline"
                onclick={() => (open = false)}
                disabled={importing}
            >
                Cancel
            </Button>
            <Button
                onclick={handleImport}
                disabled={!hasSource || !isValidId || importing}
            >
                {#if importing}
                    <RefreshCw class="size-4 animate-spin mr-2" />
                    Importing...
                {:else}
                    Import Service
                {/if}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
