<script lang="ts">
    import * as Sheet from "$lib/components/ui/sheet/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";

    let {
        open = $bindable(false),
        event = null,
    }: {
        open: boolean;
        event: any | null;
    } = $props();

    function getFormattedAttrs(evt: any) {
        if (!evt?.attributes) return "null";
        try {
            const parsed =
                typeof evt.attributes === "string"
                    ? JSON.parse(evt.attributes)
                    : evt.attributes;
            return JSON.stringify(parsed, null, 2);
        } catch {
            return evt.attributes;
        }
    }
</script>

<Sheet.Root bind:open>
    <Sheet.Content class="w-[450px] sm:max-w-xl overflow-y-auto p-6">
        <Sheet.Header class="mb-6">
            <Sheet.Title class="text-xl">Event Details</Sheet.Title>
            <Sheet.Description>
                View full JSON attributes and metadata for this event.
            </Sheet.Description>
        </Sheet.Header>

        {#if event}
            <div class="space-y-6">
                <!-- Core Info -->
                <div
                    class="grid grid-cols-4 items-center gap-4 bg-muted/40 p-4 rounded-lg border"
                >
                    <span class="text-sm font-medium text-muted-foreground"
                        >ID</span
                    >
                    <span class="col-span-3 text-sm font-mono">{event.id}</span>

                    <span class="text-sm font-medium text-muted-foreground"
                        >Timestamp</span
                    >
                    <span class="col-span-3 text-sm font-mono"
                        >{new Date(event.timestamp).toISOString()}</span
                    >

                    <span class="text-sm font-medium text-muted-foreground"
                        >Activity</span
                    >
                    <span class="col-span-3 text-sm font-medium"
                        >{event.activity}</span
                    >
                </div>

                <!-- Attributes Payload -->
                <div class="space-y-3">
                    <div
                        class="text-sm font-semibold flex items-center justify-between"
                    >
                        Attributes Payload
                        <Badge variant="outline" class="font-mono text-[10px]"
                            >JSON</Badge
                        >
                    </div>
                    <div
                        class="bg-slate-950 text-slate-50 p-6 rounded-lg overflow-x-auto text-[13px] leading-relaxed font-mono shadow-inner border border-slate-800"
                    >
                        <pre>{getFormattedAttrs(event)}</pre>
                    </div>
                </div>

                <!-- Extra Context (if any message) -->
                {#if event.message}
                    <div class="space-y-3">
                        <div class="text-sm font-semibold">Message</div>
                        <div class="bg-muted p-4 rounded-lg text-sm border">
                            {event.message}
                        </div>
                    </div>
                {/if}
            </div>
        {/if}
    </Sheet.Content>
</Sheet.Root>
