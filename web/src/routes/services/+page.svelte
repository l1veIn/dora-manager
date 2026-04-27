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
        isInstalledService,
        serviceOrigin,
        sortServicesForCatalog,
    } from "$lib/services/catalog";

    import ServiceCard from "./ServiceCard.svelte";
    import CreateServiceDialog from "./CreateServiceDialog.svelte";
    import ImportServiceDialog from "./ImportServiceDialog.svelte";

    let installedServices = $state<any[]>([]);
    let loadingInstalled = $state(true);
    let searchQuery = $state("");
    let statusFilter = $state<"all" | "installed" | "needs_install">("all");
    let originFilter = $state<"all" | "builtin" | "git" | "local">("all");
    let currentPage = $state(1);
    const pageSize = 18;
    const catalogStateKey = "dm:services:catalog-state";
    let restoredCatalogState = $state(false);
    let lastFilterSignature = $state("");

    let operations = $state<
        Record<string, "downloading" | "installing" | "uninstalling">
    >({});

    let isCreateDialogOpen = $state(false);
    let isImportDialogOpen = $state(false);
    let isDeleteDialogOpen = $state(false);
    let serviceToDelete = $state<string | null>(null);

    async function fetchInstalled() {
        loadingInstalled = true;
        try {
            installedServices = sortServicesForCatalog((await get("/services")) || []);
        } catch (e) {
            toast.error("Failed to load service catalog");
            installedServices = [];
        } finally {
            loadingInstalled = false;
        }
    }

    async function handleAction(action: string, id: string) {
        if (action === "install") {
            operations[id] = "installing";
            try {
                await post("/services/install", { id });
                toast.success(`${id} installed successfully`);
                await fetchInstalled();
            } catch (e: any) {
                toast.error(`Failed to install ${id}: ${e.message}`);
            } finally {
                delete operations[id];
            }
        } else if (action === "uninstall") {
            serviceToDelete = id;
            isDeleteDialogOpen = true;
        }
    }

    async function confirmUninstall() {
        if (!serviceToDelete) return;
        const id = serviceToDelete;
        operations[id] = "uninstalling";
        isDeleteDialogOpen = false;
        try {
            await post("/services/uninstall", { id });
            toast.success(`${id} uninstalled`);
            await fetchInstalled();
        } catch (e: any) {
            toast.error(`Failed to uninstall ${id}: ${e.message}`);
        } finally {
            delete operations[id];
            serviceToDelete = null;
        }
    }

    onMount(() => {
        try {
            const raw = localStorage.getItem(catalogStateKey);
            if (raw) {
                const saved = JSON.parse(raw);
                searchQuery =
                    typeof saved.searchQuery === "string" ? saved.searchQuery : "";
                statusFilter =
                    saved.statusFilter === "installed" ||
                    saved.statusFilter === "needs_install"
                        ? saved.statusFilter
                        : "all";
                originFilter =
                    saved.originFilter === "builtin" ||
                    saved.originFilter === "git" ||
                    saved.originFilter === "local"
                        ? saved.originFilter
                        : "all";
                currentPage =
                    typeof saved.currentPage === "number" && saved.currentPage > 0
                        ? Math.floor(saved.currentPage)
                        : 1;
            }
        } catch (e) {
            console.warn("Failed to restore service catalog state", e);
        } finally {
            restoredCatalogState = true;
        }
        fetchInstalled();
    });

    $effect(() => {
        const signature = JSON.stringify({
            searchQuery,
            statusFilter,
            originFilter,
        });

        if (!restoredCatalogState) {
            return;
        }

        if (!lastFilterSignature) {
            lastFilterSignature = signature;
            return;
        }

        if (lastFilterSignature !== signature) {
            currentPage = 1;
            lastFilterSignature = signature;
        }
    });

    $effect(() => {
        pageCount;
        if (currentPage > pageCount) {
            currentPage = pageCount;
        }
    });

    $effect(() => {
        if (!restoredCatalogState) return;
        try {
            localStorage.setItem(
                catalogStateKey,
                JSON.stringify({
                    searchQuery,
                    statusFilter,
                    originFilter,
                    currentPage,
                }),
            );
        } catch (e) {
            console.warn("Failed to persist service catalog state", e);
        }
    });

    let filteredInstalled = $derived(
        installedServices.filter((n) => {
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
                      ? isInstalledService(n)
                      : !isInstalledService(n);

            const matchesOrigin =
                originFilter === "all"
                    ? true
                    : serviceOrigin(n) === originFilter;

            return matchesSearch && matchesStatus && matchesOrigin;
        }),
    );

    let installedCount = $derived(
        installedServices.filter((n) => isInstalledService(n)).length,
    );
    let needsInstallCount = $derived(installedServices.length - installedCount);
    let pageCount = $derived(
        Math.max(1, Math.ceil(filteredInstalled.length / pageSize)),
    );
    let pagedServices = $derived(
        filteredInstalled.slice(
            (currentPage - 1) * pageSize,
            currentPage * pageSize,
        ),
    );
</script>

<div class="p-6 max-w-6xl mx-auto space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold tracking-tight">Services</h1>
        <div class="flex items-center gap-2">
            <Button
                variant="outline"
                onclick={() => (isImportDialogOpen = true)}
            >
                <Download class="size-4 mr-2" />
                Import Service
            </Button>
            <Button onclick={() => (isCreateDialogOpen = true)}>
                <Plus class="size-4 mr-2" />
                New Service
            </Button>
        </div>
    </div>

    <div class="rounded-xl border bg-muted/20 p-4 md:p-5 space-y-2">
        <p class="text-sm font-medium">
            This is the service catalog, not just a list of installed executables.
        </p>
        <p class="text-sm text-muted-foreground max-w-4xl">
            Filter builtin services, git imports, and local custom services separately.
            Use <span class="font-medium text-foreground">Install</span> when a
            service exists in the catalog but does not have a local executable yet.
            Use the service workspace to inspect code, metadata, and settings.
        </p>
        <div class="flex flex-wrap gap-2 pt-1">
            <Badge variant="outline">Total {installedServices.length}</Badge>
            <Badge variant="outline">Installed {installedCount}</Badge>
            <Badge variant="outline">Needs install {needsInstallCount}</Badge>
        </div>
    </div>

    <div class="space-y-4">
        <div class="flex items-center justify-between gap-4 flex-wrap">
            <h2 class="text-xl font-semibold">
                Service Catalog ({filteredInstalled.length})
            </h2>

            <div class="relative w-72">
                <Search
                    class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground"
                />
                <Input
                    type="search"
                    placeholder="Search services, descriptions, categories..."
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
                <h3 class="text-lg font-medium">No services found</h3>
                <p class="text-sm text-muted-foreground mt-1 max-w-sm">
                    No services match your current search or filters.
                </p>
            </div>
        {:else}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {#each pagedServices as service}
                    <ServiceCard
                        {service}
                        operation={operations[service.id]}
                        onAction={handleAction}
                        href={`/services/${service.id}`}
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

<CreateServiceDialog
    bind:open={isCreateDialogOpen}
    onCreated={async (id) => {
        await fetchInstalled();
        goto(`/services/${id}?new=1`);
    }}
/>

<ImportServiceDialog
    bind:open={isImportDialogOpen}
    onImported={() => fetchInstalled()}
/>

<AlertDialog.Root bind:open={isDeleteDialogOpen}>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>Confirm Delete</AlertDialog.Title>
            <AlertDialog.Description>
                Are you sure you want to delete <span
                    class="font-mono font-bold">{serviceToDelete}</span
                >? This action cannot be undone.
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
            <AlertDialog.Cancel
                onclick={() => {
                    isDeleteDialogOpen = false;
                    serviceToDelete = null;
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
