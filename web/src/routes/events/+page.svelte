<script lang="ts">
    import { onMount } from "svelte";
    import { get } from "$lib/api";
    import * as Table from "$lib/components/ui/table/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import {
        Download,
        RefreshCw,
        Search,
        Filter,
        ChevronLeft,
        ChevronRight,
    } from "lucide-svelte";
    import EventDetailsSheet from "./EventDetailsSheet.svelte";

    // State
    let events = $state<any[]>([]);
    let loading = $state(true);
    let totalEvents = $state(0);
    let currentPage = $state(1);
    const pageSize = 50;

    // Filters
    let sourceFilter = $state("all");
    let levelFilter = $state("all");
    let searchFilter = $state("");
    let searchInputTimer: ReturnType<typeof setTimeout>;

    // detail view
    let selectedEvent = $state<any>(null);
    let sheetOpen = $state(false);

    async function fetchEvents() {
        loading = true;
        try {
            const params = new URLSearchParams();
            if (sourceFilter !== "all") params.append("source", sourceFilter);
            if (levelFilter !== "all") params.append("level", levelFilter);
            if (searchFilter.trim())
                params.append("search", searchFilter.trim());

            const baseQuery = params.toString() ? `?${params.toString()}` : "";

            // Fetch total count and events in parallel
            params.append("limit", pageSize.toString());
            params.append("offset", ((currentPage - 1) * pageSize).toString());
            const dataQuery = `?${params.toString()}`;

            const [countRes, eventsRes] = await Promise.all([
                get(`/events/count${baseQuery}`),
                get(`/events${dataQuery}`),
            ]);

            totalEvents = (countRes as any).count || 0;
            events = (eventsRes as any[]) || [];
        } catch (e) {
            console.error("Failed to fetch events", e);
        } finally {
            loading = false;
        }
    }

    function handleFilterChange() {
        currentPage = 1;
        fetchEvents();
    }

    function handleSearchInput() {
        if (searchInputTimer) clearTimeout(searchInputTimer);
        searchInputTimer = setTimeout(() => {
            handleFilterChange();
        }, 300);
    }

    function viewDetails(evt: any) {
        selectedEvent = evt;
        sheetOpen = true;
    }

    function exportXes() {
        const params = new URLSearchParams();
        params.append("format", "xes");
        if (sourceFilter !== "all") params.append("source", sourceFilter);
        if (levelFilter !== "all") params.append("level", levelFilter);
        if (searchFilter.trim()) params.append("search", searchFilter.trim());

        window.open(`/api/events/export?${params.toString()}`, "_blank");
    }

    onMount(() => {
        fetchEvents();
    });

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
            <Button variant="outline" size="sm" onclick={fetchEvents}>
                <RefreshCw
                    class="mr-2 size-4 {loading ? 'animate-spin' : ''}"
                /> Refresh
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
                placeholder="Search events backend..."
                class="h-8 pl-8 text-xs"
                bind:value={searchFilter}
                oninput={handleSearchInput}
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
                {:else if events.length === 0}
                    <Table.Row>
                        <Table.Cell
                            colspan={5}
                            class="h-24 text-center text-muted-foreground"
                            >No events found matching criteria.</Table.Cell
                        >
                    </Table.Row>
                {:else}
                    {#each events as evt}
                        <Table.Row
                            class="cursor-pointer hover:bg-muted/50 transition-colors"
                            onclick={() => viewDetails(evt)}
                        >
                            <Table.Cell
                                class="font-mono text-[11px] text-muted-foreground"
                            >
                                {new Date(evt.timestamp).toISOString()}
                            </Table.Cell>
                            <Table.Cell>
                                <Badge
                                    variant={getLevelColor(evt.level)}
                                    class="text-[10px] uppercase font-mono px-1 py-0 h-4"
                                >
                                    {evt.level}
                                </Badge>
                            </Table.Cell>
                            <Table.Cell>
                                <Badge
                                    variant="outline"
                                    class="text-[10px] uppercase font-mono px-1 py-0 h-4 border-transparent {getSourceColor(
                                        evt.source,
                                    )}"
                                >
                                    {evt.source}
                                </Badge>
                            </Table.Cell>
                            <Table.Cell
                                class="font-medium text-sm truncate max-w-[200px]"
                                title={evt.activity}
                            >
                                {evt.activity}
                            </Table.Cell>
                            <Table.Cell
                                class="text-sm truncate max-w-md text-muted-foreground"
                                title={evt.message}
                            >
                                {evt.message || "-"}
                            </Table.Cell>
                        </Table.Row>
                    {/each}
                {/if}
            </Table.Body>
        </Table.Root>
    </div>

    <div class="flex items-center justify-between px-2 pt-2">
        <div class="text-sm text-muted-foreground">
            Showing {totalEvents === 0 ? 0 : (currentPage - 1) * pageSize + 1} to
            {Math.min(currentPage * pageSize, totalEvents)} of
            <span class="font-medium text-foreground">{totalEvents}</span> events
        </div>
        <div class="flex items-center gap-2">
            <Button
                variant="outline"
                size="sm"
                disabled={currentPage === 1 || loading}
                onclick={() => {
                    currentPage--;
                    fetchEvents();
                }}
            >
                <ChevronLeft class="size-4 mr-1" /> Previous
            </Button>
            <Button
                variant="outline"
                size="sm"
                disabled={currentPage * pageSize >= totalEvents || loading}
                onclick={() => {
                    currentPage++;
                    fetchEvents();
                }}
            >
                Next <ChevronRight class="size-4 ml-1" />
            </Button>
        </div>
    </div>
</div>

<EventDetailsSheet bind:open={sheetOpen} event={selectedEvent} />
