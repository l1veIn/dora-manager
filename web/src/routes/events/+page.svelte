<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { get } from "$lib/api";
    import * as Table from "$lib/components/ui/table/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import * as Sheet from "$lib/components/ui/sheet/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { Download, RefreshCw, Search, Filter } from "lucide-svelte";

    // State
    let events = $state<any[]>([]);
    let loading = $state(true);

    // Filters
    let sourceFilter = $state("all");
    let levelFilter = $state("all");
    let searchFilter = $state("");

    // detail view
    let selectedEvent = $state<any>(null);
    let sheetOpen = $state(false);

    // Auto-refresh timer
    let refreshTimer: ReturnType<typeof setInterval>;
    let autoRefresh = $state(true);

    async function fetchEvents() {
        loading = true;
        try {
            let query = "/events?limit=100";
            if (sourceFilter !== "all") query += `&source=${sourceFilter}`;
            if (levelFilter !== "all") query += `&level=${levelFilter}`;

            const res = (await get(query)) as any[];
            events = res || [];
        } catch (e) {
            console.error("Failed to fetch events", e);
        } finally {
            loading = false;
        }
    }

    function handleFilterChange() {
        fetchEvents();
    }

    function viewDetails(evt: any) {
        selectedEvent = evt;
        sheetOpen = true;
    }

    function exportXes() {
        let query = "/events/export?format=xes";
        if (sourceFilter !== "all") query += `&source=${sourceFilter}`;
        if (levelFilter !== "all") query += `&level=${levelFilter}`;

        // In a real app we'd download this properly, triggering a browser save
        window.open(`/api${query}`, "_blank");
    }

    onMount(() => {
        fetchEvents();
        refreshTimer = setInterval(() => {
            if (autoRefresh) fetchEvents();
        }, 5000);
    });

    onDestroy(() => {
        if (refreshTimer) clearInterval(refreshTimer);
    });

    let filteredEvents = $derived(
        events.filter((e) => {
            if (!searchFilter) return true;
            const term = searchFilter.toLowerCase();
            return (
                (e.activity || "").toLowerCase().includes(term) ||
                (e.message || "").toLowerCase().includes(term) ||
                (e.source || "").toLowerCase().includes(term)
            );
        }),
    );

    function getLevelColor(level: string) {
        switch (level?.toLowerCase()) {
            case "error":
                return "destructive";
            case "warn":
                return "outline"; // shadcn amber workaround
            case "info":
                return "default";
            case "debug":
                return "secondary";
            case "trace":
                return "secondary";
            default:
                return "outline";
        }
    }

    function getSourceColor(source: string) {
        switch (source) {
            case "core":
                return "bg-blue-500/10 text-blue-600 border-blue-500/20";
            case "server":
                return "bg-purple-500/10 text-purple-600 border-purple-500/20";
            case "dataflow":
                return "bg-green-500/10 text-green-600 border-green-500/20";
            case "frontend":
                return "bg-orange-500/10 text-orange-600 border-orange-500/20";
            default:
                return "bg-slate-500/10 text-slate-600 border-slate-500/20";
        }
    }
</script>

<div class="p-6 max-w-6xl mx-auto space-y-4 h-full flex flex-col">
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-3xl font-bold tracking-tight">Events</h1>
            <p class="text-sm text-muted-foreground">
                Real-time system observability and logs.
            </p>
        </div>

        <div class="flex items-center gap-2">
            <Button variant="outline" size="sm" onclick={exportXes}>
                <Download class="mr-2 size-4" /> Export XES
            </Button>
            <Button
                variant={autoRefresh ? "secondary" : "outline"}
                size="sm"
                onclick={() => (autoRefresh = !autoRefresh)}
                class="w-32"
            >
                <RefreshCw
                    class="mr-2 size-4 {autoRefresh ? 'animate-spin' : ''}"
                />
                {autoRefresh ? "Auto-refresh On" : "Auto-refresh Off"}
            </Button>
        </div>
    </div>

    <div class="flex items-center gap-4 bg-muted/40 p-2 rounded-lg border">
        <div class="flex items-center gap-2 text-sm text-muted-foreground mr-2">
            <Filter class="size-4" /> Filters:
        </div>

        <div class="w-40">
            <Select.Root
                type="single"
                bind:value={sourceFilter}
                onValueChange={handleFilterChange}
            >
                <Select.Trigger class="h-8 text-xs">
                    {sourceFilter === "all" ? "All Sources" : sourceFilter}
                </Select.Trigger>
                <Select.Content>
                    <Select.Item value="all">All Sources</Select.Item>
                    <Select.Item value="core">core</Select.Item>
                    <Select.Item value="server">server</Select.Item>
                    <Select.Item value="dataflow">dataflow</Select.Item>
                    <Select.Item value="frontend">frontend</Select.Item>
                </Select.Content>
            </Select.Root>
        </div>

        <div class="w-32">
            <Select.Root
                type="single"
                bind:value={levelFilter}
                onValueChange={handleFilterChange}
            >
                <Select.Trigger class="h-8 text-xs">
                    {levelFilter === "all" ? "All Levels" : levelFilter}
                </Select.Trigger>
                <Select.Content>
                    <Select.Item value="all">All Levels</Select.Item>
                    <Select.Item value="info">info</Select.Item>
                    <Select.Item value="warn">warn</Select.Item>
                    <Select.Item value="error">error</Select.Item>
                    <Select.Item value="debug">debug</Select.Item>
                </Select.Content>
            </Select.Root>
        </div>

        <div class="relative flex-1 max-w-sm ml-auto">
            <Search
                class="absolute left-2.5 top-2 h-4 w-4 text-muted-foreground"
            />
            <Input
                type="search"
                placeholder="Filter results..."
                class="h-8 pl-8 text-xs"
                bind:value={searchFilter}
            />
        </div>
    </div>

    <div class="border rounded-md shrink-0 overflow-auto bg-card flex-1">
        <Table.Root>
            <Table.Header class="sticky top-0 bg-card z-10 shadow-sm">
                <Table.Row>
                    <Table.Head class="w-[180px]">Timestamp</Table.Head>
                    <Table.Head class="w-[100px]">Level</Table.Head>
                    <Table.Head class="w-[120px]">Source</Table.Head>
                    <Table.Head class="w-[200px]">Activity</Table.Head>
                    <Table.Head>Message</Table.Head>
                </Table.Row>
            </Table.Header>
            <Table.Body>
                {#if loading && events.length === 0}
                    <Table.Row>
                        <Table.Cell colspan={5} class="h-24 text-center"
                            >Loading events...</Table.Cell
                        >
                    </Table.Row>
                {:else if filteredEvents.length === 0}
                    <Table.Row>
                        <Table.Cell
                            colspan={5}
                            class="h-24 text-center text-muted-foreground"
                            >No events found matching criteria.</Table.Cell
                        >
                    </Table.Row>
                {:else}
                    {#each filteredEvents as evt}
                        <Table.Row
                            class="cursor-pointer hover:bg-muted/50 transition-colors"
                            onclick={() => viewDetails(evt)}
                        >
                            <Table.Cell
                                class="font-mono text-[11px] text-muted-foreground"
                                >{new Date(
                                    evt.timestamp,
                                ).toISOString()}</Table.Cell
                            >
                            <Table.Cell>
                                <Badge
                                    variant={getLevelColor(evt.level)}
                                    class="text-[10px] uppercase font-mono px-1 py-0 h-4"
                                    >{evt.level}</Badge
                                >
                            </Table.Cell>
                            <Table.Cell>
                                <Badge
                                    variant="outline"
                                    class="text-[10px] uppercase font-mono px-1 py-0 h-4 border-transparent {getSourceColor(
                                        evt.source,
                                    )}">{evt.source}</Badge
                                >
                            </Table.Cell>
                            <Table.Cell
                                class="font-medium text-sm truncate max-w-[200px]"
                                title={evt.activity}>{evt.activity}</Table.Cell
                            >
                            <Table.Cell
                                class="text-sm truncate max-w-md text-muted-foreground"
                                title={evt.message}
                                >{evt.message || "-"}</Table.Cell
                            >
                        </Table.Row>
                    {/each}
                {/if}
            </Table.Body>
        </Table.Root>
    </div>
</div>

<Sheet.Root bind:open={sheetOpen}>
    <Sheet.Content class="w-[400px] sm:w-[540px] overflow-y-auto">
        <Sheet.Header>
            <Sheet.Title>Event Details</Sheet.Title>
            <Sheet.Description>
                View full JSON attributes and metadata for this event.
            </Sheet.Description>
        </Sheet.Header>

        {#if selectedEvent}
            <div class="mt-6 space-y-4">
                <div class="grid grid-cols-4 items-center gap-4">
                    <span class="text-sm font-medium text-muted-foreground"
                        >ID</span
                    >
                    <span class="col-span-3 text-sm font-mono"
                        >{selectedEvent.id_}</span
                    >
                </div>
                <div class="grid grid-cols-4 items-center gap-4">
                    <span class="text-sm font-medium text-muted-foreground"
                        >Timestamp</span
                    >
                    <span class="col-span-3 text-sm font-mono"
                        >{new Date(selectedEvent.timestamp).toISOString()}</span
                    >
                </div>
                <div class="grid grid-cols-4 items-center gap-4">
                    <span class="text-sm font-medium text-muted-foreground"
                        >Activity</span
                    >
                    <span class="col-span-3 text-sm font-medium"
                        >{selectedEvent.activity}</span
                    >
                </div>

                <Separator />

                <div>
                    <div class="text-sm font-medium mb-2">
                        Attributes Payload
                    </div>
                    <div
                        class="bg-slate-950 text-slate-50 p-4 rounded-md overflow-x-auto text-xs font-mono"
                    >
                        <pre>{JSON.stringify(
                                selectedEvent.attributes,
                                null,
                                2,
                            )}</pre>
                    </div>
                </div>
            </div>
        {/if}
    </Sheet.Content>
</Sheet.Root>
