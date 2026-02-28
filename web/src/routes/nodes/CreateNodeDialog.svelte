<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Textarea } from "$lib/components/ui/textarea/index.js";
    import { toast } from "svelte-sonner";
    import { post } from "$lib/api";
    import { RefreshCw, Code2 } from "lucide-svelte";

    let { open = $bindable(false), onCreated } = $props<{
        open: boolean;
        onCreated: () => void;
    }>();

    let id = $state("");
    let description = $state("");
    let creating = $state(false);

    // Validate ID (kebab-case mostly)
    let isValidId = $derived(id.length > 0 && /^[a-z0-9-]+$/.test(id));

    async function handleCreate() {
        if (!isValidId) {
            toast.error(
                "Node ID must contain only lowercase letters, numbers, and hyphens",
            );
            return;
        }

        creating = true;
        try {
            await post("/nodes/create", { id, description });
            toast.success(`Node scaffold '${id}' created!`);
            open = false; // Close dialog
            // Reset form
            id = "";
            description = "";
            // Notify parent
            onCreated();
        } catch (e: any) {
            toast.error(`Failed to create node: ${e.message}`);
        } finally {
            creating = false;
        }
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2">
                <Code2 class="size-5 text-primary" />
                New Python Node
            </Dialog.Title>
            <Dialog.Description>
                Create a new local Dora node scaffold. This generates a
                pyproject.toml, main.py template, and sets up an empty dm.json
                for metadata tracking.
            </Dialog.Description>
        </Dialog.Header>

        <div class="grid gap-4 py-4">
            <div class="grid gap-2">
                <Label for="node-id" class="font-medium">Node ID</Label>
                <Input
                    id="node-id"
                    bind:value={id}
                    placeholder="e.g., my-awesome-filter"
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

            <div class="grid gap-2">
                <Label for="node-desc" class="font-medium"
                    >Description (Optional)</Label
                >
                <Textarea
                    id="node-desc"
                    bind:value={description}
                    placeholder="What does this node do?"
                    class="h-20 resize-none"
                />
            </div>
        </div>

        <Dialog.Footer>
            <Button
                variant="outline"
                onclick={() => (open = false)}
                disabled={creating}
            >
                Cancel
            </Button>
            <Button onclick={handleCreate} disabled={!isValidId || creating}>
                {#if creating}
                    <RefreshCw class="size-4 animate-spin mr-2" />
                    Creating...
                {:else}
                    Create Node
                {/if}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
