<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Search, Package, Plus, Download } from "lucide-svelte";
    import { toast } from "svelte-sonner";

    import NodeCard from "./NodeCard.svelte";
    import CreateNodeDialog from "./CreateNodeDialog.svelte";
    import ImportNodeDialog from "./ImportNodeDialog.svelte";

    let installedNodes = $state<any[]>([]);
    let loadingInstalled = $state(true);
    let searchQuery = $state("");

    // Use a record for operations to handle concurrent actions
    let operations = $state<
        Record<string, "downloading" | "installing" | "uninstalling">
    >({});

    let isCreateDialogOpen = $state(false);
    let isImportDialogOpen = $state(false);
    let isDeleteDialogOpen = $state(false);
    let nodeToDelete = $state<string | null>(null);

    let selectedNode = $state<any | null>(null);

    async function fetchInstalled() {
        try {
            installedNodes = (await get("/nodes")) || [];
        } catch (e) {
            toast.error("Failed to load installed nodes");
            installedNodes = [];
        } finally {
            loadingInstalled = false;
        }
    }

    async function handleAction(action: string, id: string) {
        if (action === "download") {
            operations[id] = "downloading";
            try {
                await post("/nodes/download", { id });
                toast.success(`${id} downloaded successfully`);
                await fetchInstalled();
            } catch (e: any) {
                toast.error(`Failed to download ${id}: ${e.message}`);
            } finally {
                delete operations[id];
            }
        } else if (action === "install") {
            operations[id] = "installing";
            try {
                await post("/nodes/install", { id });
                toast.success(`${id} installed successfully`);
                await fetchInstalled();
            } catch (e: any) {
                toast.error(`Failed to install ${id}: ${e.message}`);
            } finally {
                delete operations[id];
            }
        } else if (action === "uninstall") {
            nodeToDelete = id;
            isDeleteDialogOpen = true;
        }
    }

    async function confirmUninstall() {
        if (!nodeToDelete) return;
        const id = nodeToDelete;
        operations[id] = "uninstalling";
        isDeleteDialogOpen = false;
        try {
            await post("/nodes/uninstall", { id });
            toast.success(`${id} uninstalled`);
            await fetchInstalled();
        } catch (e: any) {
            toast.error(`Failed to uninstall ${id}: ${e.message}`);
        } finally {
            delete operations[id];
            nodeToDelete = null;
        }
    }

    onMount(() => {
        fetchInstalled();
    });

    let filteredInstalled = $derived(
        installedNodes.filter(
            (n) =>
                (n.name || n.id || "")
                    .toLowerCase()
                    .includes(searchQuery.toLowerCase()) ||
                (n.description || "")
                    .toLowerCase()
                    .includes(searchQuery.toLowerCase()),
        ),
    );

    let installedIds = $derived(new Set(installedNodes.map((n) => n.id)));
</script>

<div class="p-6 max-w-6xl mx-auto space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold tracking-tight">Nodes</h1>
        <div class="flex items-center gap-2">
            <Button
                variant="outline"
                onclick={() => (isImportDialogOpen = true)}
            >
                <Download class="size-4 mr-2" />
                Import Node
            </Button>
            <Button onclick={() => (isCreateDialogOpen = true)}>
                <Plus class="size-4 mr-2" />
                New Node
            </Button>
        </div>
    </div>

    <div class="mt-8">
        <div class="flex items-center justify-between mb-4 gap-4">
            <h2 class="text-xl font-semibold">
                Installed Nodes ({installedNodes.length})
            </h2>

            <div class="relative w-72">
                <Search
                    class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground"
                />
                <Input
                    type="search"
                    placeholder="Search nodes..."
                    class="pl-8"
                    bind:value={searchQuery}
                />
            </div>
        </div>

        <div class="mt-4">
            {#if loadingInstalled}
                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {#each Array(3) as _}
                        <div
                            class="animate-pulse h-36 bg-muted/50 rounded-lg"
                        ></div>
                    {/each}
                </div>
            {:else if filteredInstalled.length === 0}
                <div
                    class="flex flex-col items-center justify-center p-12 text-center border rounded-lg bg-muted/10 h-64 border-dashed"
                >
                    <Package
                        class="h-12 w-12 text-muted-foreground mb-4 opacity-50"
                    />
                    <h3 class="text-lg font-medium">No nodes found</h3>
                    <p class="text-sm text-muted-foreground mt-1 max-w-sm">
                        You don't have any nodes matching your search, or none
                        are installed yet.
                    </p>
                </div>
            {:else}
                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {#each filteredInstalled as node}
                        <NodeCard
                            {node}
                            operation={operations[node.id]}
                            onAction={handleAction}
                            href={`/nodes/${node.id}`}
                        />
                    {/each}
                </div>
            {/if}
        </div>
    </div>
</div>

<!-- Modals & Sheets -->
<CreateNodeDialog
    bind:open={isCreateDialogOpen}
    onCreated={() => fetchInstalled()}
/>

<ImportNodeDialog
    bind:open={isImportDialogOpen}
    onImported={() => fetchInstalled()}
/>

<AlertDialog.Root bind:open={isDeleteDialogOpen}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>Confirm Delete</AlertDialog.Title>
            <AlertDialog.Description>
                Are you sure you want to delete <span
                    class="font-mono font-bold">{nodeToDelete}</span
                >? This action cannot be undone.
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel
                onclick={() => {
                    isDeleteDialogOpen = false;
                    nodeToDelete = null;
                }}
            >
                Cancel
            </AlertDialog.Cancel>
            <AlertDialog.Action onclick={confirmUninstall}>
                Delete
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>
