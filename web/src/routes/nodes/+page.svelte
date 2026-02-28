<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Search, Package, Plus } from "lucide-svelte";
    import { toast } from "svelte-sonner";

    import NodeCard from "./NodeCard.svelte";
    import NodeDetails from "./NodeDetails.svelte";
    import CreateNodeDialog from "./CreateNodeDialog.svelte";

    let installedNodes = $state<any[]>([]);
    let registryNodes = $state<any[]>([]);
    let loadingInstalled = $state(true);
    let loadingRegistry = $state(true);
    let searchQuery = $state("");

    // Use a record for operations to handle concurrent actions
    let operations = $state<
        Record<string, "downloading" | "installing" | "uninstalling">
    >({});

    // Dialog & Sheet state
    let isCreateDialogOpen = $state(false);
    let isDetailsSheetOpen = $state(false);
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

    async function fetchRegistry() {
        try {
            registryNodes = (await get("/registry")) || [];
        } catch (e) {
            toast.error("Failed to load node registry");
            registryNodes = [];
        } finally {
            loadingRegistry = false;
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
            if (!confirm(`Are you sure you want to uninstall ${id}?`)) return;
            operations[id] = "uninstalling";
            try {
                await post("/nodes/uninstall", { id });
                toast.success(`${id} uninstalled`);
                await fetchInstalled();
            } catch (e: any) {
                toast.error(`Failed to uninstall ${id}: ${e.message}`);
            } finally {
                delete operations[id];
            }
        }
    }

    function viewDetails(node: any) {
        selectedNode = node;
        isDetailsSheetOpen = true;
    }

    onMount(() => {
        fetchInstalled();
        fetchRegistry();
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

    let filteredRegistry = $derived(
        registryNodes.filter(
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

<div class="p-6 max-w-6xl mx-auto space-y-6 h-full flex flex-col">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold tracking-tight">Nodes</h1>
        <Button onclick={() => (isCreateDialogOpen = true)}>
            <Plus class="size-4 mr-2" />
            New Node
        </Button>
    </div>

    <Tabs.Root value="installed" class="flex-1 flex flex-col">
        <div class="flex items-center justify-between mb-4 gap-4">
            <Tabs.List>
                <Tabs.Trigger value="installed"
                    >Installed ({installedNodes.length})</Tabs.Trigger
                >
                <Tabs.Trigger value="registry">Registry</Tabs.Trigger>
            </Tabs.List>

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

        <Tabs.Content value="installed" class="flex-1 mt-0">
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
                            isRegistry={false}
                            isInstalled={true}
                            operation={operations[node.id]}
                            onAction={handleAction}
                            onViewDetails={viewDetails}
                        />
                    {/each}
                </div>
            {/if}
        </Tabs.Content>

        <Tabs.Content value="registry" class="flex-1 mt-0">
            {#if loadingRegistry}
                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {#each Array(3) as _}
                        <div
                            class="animate-pulse h-36 bg-muted/50 rounded-lg"
                        ></div>
                    {/each}
                </div>
            {:else if filteredRegistry.length === 0}
                <div
                    class="flex flex-col items-center justify-center p-12 text-center border rounded-lg bg-muted/10 h-64 border-dashed"
                >
                    <p class="text-muted-foreground">
                        No nodes found in the remote registry.
                    </p>
                </div>
            {:else}
                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {#each filteredRegistry as node}
                        {@const installedData = installedNodes.find(
                            (n) => n.id === node.id,
                        )}
                        <NodeCard
                            node={installedData || node}
                            isRegistry={true}
                            isInstalled={!!installedData}
                            operation={operations[node.id]}
                            onAction={handleAction}
                            onViewDetails={viewDetails}
                        />
                    {/each}
                </div>
            {/if}
        </Tabs.Content>
    </Tabs.Root>
</div>

<!-- Modals & Sheets -->
<CreateNodeDialog
    bind:open={isCreateDialogOpen}
    onCreated={() => fetchInstalled()}
/>

<NodeDetails bind:open={isDetailsSheetOpen} node={selectedNode} />
