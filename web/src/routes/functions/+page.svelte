<script lang="ts">
    import { onMount } from "svelte";
    import { get } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Box, Terminal, Search } from "lucide-svelte";
    import { Input } from "$lib/components/ui/input/index.js";
    import { goto } from "$app/navigation";

    let functions = $state<any[]>([]);
    let loading = $state(true);
    let searchQuery = $state("");

    async function fetchFunctions() {
        loading = true;
        try {
            functions = (await get("/fn")) || [];
        } catch (e: any) {
            console.error("Failed to load functions", e);
            functions = [];
        } finally {
            loading = false;
        }
    }

    function invokeNow(id: string, method: string) {
        goto(`/functions/${id}?invoke=${method}`);
    }

    onMount(fetchFunctions);

    let filteredFunctions = $derived(
        functions.filter((f) =>
            (f.name || f.id || "")
                .toLowerCase()
                .includes(searchQuery.toLowerCase()),
        ),
    );
</script>

<div class="p-6 max-w-6xl mx-auto space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold tracking-tight">Functions</h1>
    </div>

    <div class="rounded-xl border bg-muted/20 p-4 md:p-5 space-y-2">
        <p class="text-sm font-medium">
            Functions are callable services running as Python subprocesses.
        </p>
        <p class="text-sm text-muted-foreground max-w-4xl">
            Each function exposes one or more methods. Click to view details,
            inspect the entry script, or invoke a method directly.
        </p>
        <div class="flex flex-wrap gap-2 pt-1">
            <Badge variant="outline">Total {functions.length}</Badge>
        </div>
    </div>

    <div class="space-y-4">
        <div class="flex items-center justify-between gap-4 flex-wrap">
            <h2 class="text-xl font-semibold">
                Function Catalog ({filteredFunctions.length})
            </h2>
            <div class="relative w-72">
                <Search
                    class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground"
                />
                <Input
                    type="search"
                    placeholder="Search functions..."
                    class="pl-8"
                    bind:value={searchQuery}
                />
            </div>
        </div>

        {#if loading}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {#each Array(3) as _}
                    <div class="animate-pulse h-36 bg-muted/50 rounded-lg"></div>
                {/each}
            </div>
        {:else if filteredFunctions.length === 0}
            <div
                class="flex flex-col items-center justify-center p-12 text-center border rounded-lg bg-muted/10 h-64 border-dashed"
            >
                <Box
                    class="h-12 w-12 text-muted-foreground mb-4 opacity-50"
                />
                <h3 class="text-lg font-medium">No functions found</h3>
                <p class="text-sm text-muted-foreground mt-1 max-w-sm">
                    {#if searchQuery}
                        No functions match your search.
                    {:else}
                        No functions are registered. Add a service.json to
                        ~/.dm/functions/ to get started.
                    {/if}
                </p>
            </div>
        {:else}
            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {#each filteredFunctions as fn}
                    <div
                        class="rounded-lg border bg-card text-card-foreground shadow-sm hover:shadow-md transition-shadow cursor-pointer"
                        onclick={() => goto(`/functions/${fn.id}`)}
                    >
                        <div class="p-5 space-y-3">
                            <div class="flex items-start justify-between">
                                <div class="flex items-center gap-2 min-w-0">
                                    <Box class="size-4 shrink-0 text-primary" />
                                    <span class="font-semibold truncate">
                                        {fn.name || fn.id}
                                    </span>
                                </div>
                                <Badge variant="secondary" class="shrink-0 ml-2">
                                    v{fn.version}
                                </Badge>
                            </div>
                            {#if fn.description}
                                <p class="text-sm text-muted-foreground line-clamp-2">
                                    {fn.description}
                                </p>
                            {/if}
                            <div class="flex flex-wrap gap-1.5 pt-1">
                                {#each fn.methods as method}
                                    <Badge variant="outline" class="text-xs">
                                        {method.name}
                                    </Badge>
                                {/each}
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</div>
