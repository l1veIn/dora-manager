<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { get, post } from "$lib/api";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Search, Package, Plus, Download } from "lucide-svelte";
    import { toast } from "svelte-sonner";
    import {
        isInstalledNode,
        nodeOrigin,
        sortNodesForCatalog,
    } from "$lib/nodes/catalog";

    import NodeCard from "./NodeCard.svelte";
    import CreateNodeDialog from "./CreateNodeDialog.svelte";
    import ImportNodeDialog from "./ImportNodeDialog.svelte";

    let installedNodes = $state<any[]>([]);
    let loadingInstalled = $state(true);
    let searchQuery = $state("");
    let statusFilter = $state<"all" | "installed" | "needs_install">("all");
    let originFilter = $state<"all" | "builtin" | "git" | "local">("all");
    let currentPage = $state(1);
    const pageSize = 18;

    let operations = $state<
        Record<string, "downloading" | "installing" | "uninstalling">
    >({});

    let isCreateDialogOpen = $state(false);
    let isImportDialogOpen = $state(false);
    let isDeleteDialogOpen = $state(false);
    let nodeToDelete = $state<string | null>(null);

    async function fetchInstalled() {
        loadingInstalled = true;
        try {
            installedNodes = sortNodesForCatalog((await get("/nodes")) || []);
        } catch (e) {
            toast.error("Failed to load node catalog");
            installedNodes = [];
        } finally {
            loadingInstalled = false;
        }
    }

    async function handleAction(action: string, id: string) {
        if (action === "install") {
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

    $effect(() => {
        searchQuery;
        statusFilter;
        originFilter;
        currentPage = 1;
    });

    let filteredInstalled = $derived(
        installedNodes.filter((n) => {
            const matchesSearch =
                (n.name || n.id || "")
                    .toLowerCase()
                    .includes(searchQuery.toLowerCase()) ||
                (n.description || "")
                    .toLowerCase()
                    .includes(searchQuery.toLowerCase()) ||
                (n.display?.category || "")
                    .toLowerCase()
                    .includes(searchQuery.toLowerCase());

            const matchesStatus =
                statusFilter === "all"
                    ? true
                    : statusFilter === "installed"
                      ? isInstalledNode(n)
                      : !isInstalledNode(n);

            const matchesOrigin =
                originFilter === "all"
                    ? true
                    : nodeOrigin(n) === originFilter;

            return matchesSearch && matchesStatus && matchesOrigin;
        }),
    );

    let installedCount = $derived(
        installedNodes.filter((n) => isInstalledNode(n)).length,
    );
    let needsInstallCount = $derived(installedNodes.length - installedCount);
    let pageCount = $derived(
        Math.max(1, Math.ceil(filteredInstalled.length / pageSize)),
    );
    let pagedNodes = $derived(
        filteredInstalled.slice(
            (currentPage - 1) * pageSize,
            currentPage * pageSize,
        ),
    );
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

    <div class="rounded-xl border bg-muted/20 p-4 md:p-5 space-y-2">
        <p class="text-sm font-medium">
            This is the node catalog, not just a list of installed executables.
        </p>
        <p class="text-sm text-muted-foreground max-w-4xl">
            Filter builtin nodes, git imports, and local custom nodes separately.
            Use <span class="font-medium text-foreground">Install</span> when a
            node exists in the catalog but does not have a local executable yet.
            Use the node workspace to inspect code, metadata, and settings.
        </p>
        <div class="flex flex-wrap gap-2 pt-1">
            <Badge variant="outline">Total {installedNodes.length}</Badge>
            <Badge variant="outline">Installed {installedCount}</Badge>
            <Badge variant="outline">Needs install {needsInstallCount}</Badge>
        </div>
    </div>

    <div class="space-y-4">
        <div class="flex items-center justify-between gap-4 flex-wrap">
            <h2 class="text-xl font-semibold">
                Node Catalog ({filteredInstalled.length})
            </h2>

            <div class="relative w-72">
                <Search
                    class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground"
                />
                <Input
                    type="search"
                    placeholder="Search nodes, descriptions, categories..."
                    class="pl-8"
                    bind:value={searchQuery}
                />
            </div>
        </div>

        <div class="flex flex-wrap items-center gap-2">
            <Button
                variant={statusFilter === "all" ? "default" : "outline"}
                size="sm"
                onclick={() => (statusFilter = "all")}
            >
                All
            </Button>
            <Button
                variant={statusFilter === "installed" ? "default" : "outline"}
                size="sm"
                onclick={() => (statusFilter = "installed")}
            >
                Installed
            </Button>
            <Button
                variant={statusFilter === "needs_install" ? "default" : "outline"}
                size="sm"
                onclick={() => (statusFilter = "needs_install")}
            >
                Needs Install
            </Button>
            <div class="mx-2 h-6 w-px bg-border"></div>
            <Button
                variant={originFilter === "all" ? "default" : "outline"}
                size="sm"
                onclick={() => (originFilter = "all")}
            >
                Any Origin
            </Button>
            <Button
                variant={originFilter === "builtin" ? "default" : "outline"}
                size="sm"
                onclick={() => (originFilter = "builtin")}
            >
                Builtin
            </Button>
            <Button
                variant={originFilter === "git" ? "default" : "outline"}
                size="sm"
                onclick={() => (originFilter = "git")}
            >
                Git Import
            </Button>
            <Button
                variant={originFilter === "local" ? "default" : "outline"}
                size="sm"
                onclick={() => (originFilter = "local")}
            >
                Local
            </Button>
        </div>

        {#if loadingInstalled}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {#each Array(6) as _}
                    <div class="animate-pulse h-44 bg-muted/50 rounded-lg"></div>
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
                    No nodes match your current search or filters.
                </p>
            </div>
        {:else}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {#each pagedNodes as node}
                    <NodeCard
                        {node}
                        operation={operations[node.id]}
                        onAction={handleAction}
                        href={`/nodes/${node.id}`}
                    />
                {/each}
            </div>

            {#if pageCount > 1}
                <div class="flex items-center justify-between pt-2">
                    <p class="text-sm text-muted-foreground">
                        Page {currentPage} of {pageCount}
                    </p>
                    <div class="flex items-center gap-2">
                        <Button
                            variant="outline"
                            size="sm"
                            disabled={currentPage <= 1}
                            onclick={() =>
                                (currentPage = Math.max(1, currentPage - 1))}
                        >
                            Previous
                        </Button>
                        <Button
                            variant="outline"
                            size="sm"
                            disabled={currentPage >= pageCount}
                            onclick={() =>
                                (currentPage = Math.min(pageCount, currentPage + 1))}
                        >
                            Next
                        </Button>
                    </div>
                </div>
            {/if}
        {/if}
    </div>
</div>

<CreateNodeDialog
    bind:open={isCreateDialogOpen}
    onCreated={async (id) => {
        await fetchInstalled();
        goto(`/nodes/${id}?new=1`);
    }}
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
