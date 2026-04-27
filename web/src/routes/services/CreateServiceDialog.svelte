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
        onCreated: (id: string) => void;
    }>();

    let id = $state("");
    let description = $state("");
    let creating = $state(false);

    // Validate ID (kebab-case mostly)
    let isValidId = $derived(id.length > 0 && /^[a-z0-9-]+$/.test(id));
    let scaffoldPath = $derived(
        id ? `~/.dm/services/${id}` : "~/.dm/services/<service-id>",
    );

    async function handleCreate() {
        if (!isValidId) {
            toast.error(
                "Service ID must contain only lowercase letters, numbers, and hyphens",
            );
            return;
        }

        creating = true;
        try {
            const createdId = id;
            await post("/services/create", { id, description });
            toast.success(
                `Service scaffold '${createdId}' created at ~/.dm/services/${createdId}`,
            );
            open = false; // Close dialog
            // Reset form
            id = "";
            description = "";
            // Notify parent
            onCreated(createdId);
        } catch (e: any) {
            toast.error(`Failed to create service: ${e.message}`);
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
                New Python Service
            </Dialog.Title>
            <Dialog.Description>
                Create a new local Dora service scaffold under
                <span class="font-mono">~/.dm/services/&lt;service-id&gt;</span>, not
                the repo tree. This generates a pyproject.toml, main.py
                template, README, config.json path, and service.json metadata.
            </Dialog.Description>
        </Dialog.Header>

        <div class="grid gap-4 py-4">
            <div class="grid gap-2">
                <Label for="service-id" class="font-medium">Service ID</Label>
                <Input
                    id="service-id"
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
                {:else}
                    <p class="text-xs text-muted-foreground">
                        Scaffold location:
                        <span class="font-mono">{scaffoldPath}</span>
                    </p>
                {/if}
            </div>

            <div class="grid gap-2">
                <Label for="service-desc" class="font-medium"
                    >Description (Optional)</Label
                >
                <Textarea
                    id="service-desc"
                    bind:value={description}
                    placeholder="What does this service do?"
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
                    Create Service
                {/if}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
