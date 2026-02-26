<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import {
        Search,
        Download,
        Trash2,
        Package,
        RefreshCw,
    } from "lucide-svelte";
    import { toast } from "svelte-sonner";

    let installedNodes = $state<any[]>([]);
    let registryNodes = $state<any[]>([]);
    let loadingInstalled = $state(true);
    let loadingRegistry = $state(true);
    let searchQuery = $state("");

    // Track ongoing operations
    let operations = $state<Record<string, "installing" | "uninstalling">>({});

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

    async function installNode(id: string) {
        operations[id] = "installing";
        toast.info(`Installing ${id}...`);
        try {
            await post("/nodes/install", { id });
            toast.success(`${id} installed successfully`);
            await fetchInstalled();
        } catch (e: any) {
            toast.error(`Failed to install ${id}: ${e.message}`);
        } finally {
            delete operations[id];
        }
    }

    async function uninstallNode(id: string) {
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
                <div class="space-y-4">
                    {#each Array(3) as _}
                        <div
                            class="animate-pulse h-24 bg-muted/50 rounded-lg"
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
                        <div
                            class="border rounded-lg p-5 flex flex-col bg-card hover:border-slate-300 dark:hover:border-slate-700 transition-colors duration-200"
                        >
                            <div class="flex items-start justify-between">
                                <div
                                    class="font-bold font-mono text-base truncate pr-2"
                                >
                                    {node.name || node.id}
                                </div>
                                <Badge
                                    variant="outline"
                                    class="font-mono text-[10px] whitespace-nowrap"
                                    >{node.version || "v0.0.0"}</Badge
                                >
                            </div>
                            <div
                                class="text-sm text-muted-foreground mt-2 line-clamp-2 h-10"
                            >
                                {node.description || "No description provided."}
                            </div>

                            <Separator class="my-4" />

                            <div
                                class="mt-auto flex items-center justify-between"
                            >
                                <div class="flex items-center gap-2">
                                    <Badge
                                        variant="secondary"
                                        class="text-[10px]"
                                        >{node.language || "Rust"}</Badge
                                    >
                                </div>
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    class="text-red-500 hover:text-red-600 hover:bg-red-500/10 h-8 px-2"
                                    disabled={operations[node.id] ===
                                        "uninstalling"}
                                    onclick={() => uninstallNode(node.id)}
                                >
                                    {#if operations[node.id] === "uninstalling"}
                                        <RefreshCw
                                            class="size-4 animate-spin mr-1"
                                        />
                                        Removing
                                    {:else}
                                        <Trash2 class="size-4 mr-1" />
                                        Uninstall
                                    {/if}
                                </Button>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </Tabs.Content>

        <Tabs.Content value="registry" class="flex-1 mt-0">
            {#if loadingRegistry}
                <div class="space-y-4">
                    {#each Array(3) as _}
                        <div
                            class="animate-pulse h-24 bg-muted/50 rounded-lg"
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
                        {@const isInstalled = installedIds.has(node.id)}
                        <div
                            class="border rounded-lg p-5 flex flex-col bg-card {isInstalled
                                ? 'opacity-70'
                                : ''}"
                        >
                            <div class="flex items-start justify-between">
                                <div
                                    class="font-bold font-mono text-base truncate pr-2"
                                >
                                    {node.name || node.id}
                                </div>
                                <div
                                    class="text-xs font-mono text-muted-foreground"
                                >
                                    ★ {node.stars || 0}
                                </div>
                            </div>
                            <div
                                class="text-sm text-muted-foreground mt-2 line-clamp-2 h-10"
                            >
                                {node.description || "No description provided."}
                            </div>

                            <Separator class="my-4" />

                            <div
                                class="mt-auto flex items-center justify-between"
                            >
                                <Badge variant="secondary" class="text-[10px]"
                                    >{node.language || "Unknown"}</Badge
                                >

                                <Button
                                    variant={isInstalled
                                        ? "outline"
                                        : "default"}
                                    size="sm"
                                    class="h-8 px-3"
                                    disabled={isInstalled ||
                                        operations[node.id] === "installing"}
                                    onclick={() => installNode(node.id)}
                                >
                                    {#if isInstalled}
                                        Installed ✓
                                    {:else if operations[node.id] === "installing"}
                                        <RefreshCw
                                            class="size-3 animate-spin mr-2"
                                        />
                                        Installing...
                                    {:else}
                                        <Download class="size-3 mr-2" />
                                        Install
                                    {/if}
                                </Button>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </Tabs.Content>
    </Tabs.Root>
</div>
