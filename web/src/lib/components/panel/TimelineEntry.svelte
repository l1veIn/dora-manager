<script lang="ts">
    import type { Asset } from "./types";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { API_BASE } from "$lib/api";

    let { asset, runId } = $props<{
        asset: Asset;
        runId: string;
    }>();

    function formatTime(ts: string) {
        const d = new Date(ts);
        return d.toLocaleTimeString("en-US", {
            hour12: false,
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
        });
    }

    // Detect content type category
    function typeCategory(type: string): "text" | "image" | "json" | "raw" {
        if (type === "text/plain") return "text";
        if (type.startsWith("image/")) return "image";
        if (type === "application/json") return "json";
        return "raw";
    }

    let category = $derived(typeCategory(asset.type));

    // For JSON, try to pretty-print
    let jsonPretty = $derived(() => {
        if (category !== "json" || !asset.data) return "";
        try {
            return JSON.stringify(JSON.parse(asset.data), null, 2);
        } catch {
            return asset.data;
        }
    });

    let isJsonExpanded = $state(false);
</script>

<div
    class="group flex gap-3 px-4 py-2.5 hover:bg-muted/40 transition-colors border-b border-border/40 last:border-b-0"
>
    <!-- Timestamp -->
    <div class="shrink-0 pt-0.5">
        <span
            class="text-[11px] font-mono text-muted-foreground/60 tabular-nums"
        >
            {formatTime(asset.timestamp)}
        </span>
    </div>

    <!-- Source badge -->
    <div class="shrink-0 pt-0.5 w-32">
        <Badge
            variant="outline"
            class="text-[10px] font-mono px-1.5 py-0 h-5 truncate max-w-full {asset.input_id.includes(
                'whisper',
            )
                ? 'border-sky-500/40 text-sky-400'
                : asset.input_id.includes('qwen')
                  ? 'border-emerald-500/40 text-emerald-400'
                  : 'border-border text-muted-foreground'}"
        >
            {asset.input_id}
        </Badge>
    </div>

    <!-- Content -->
    <div class="flex-1 min-w-0">
        {#if category === "text"}
            <p
                class="text-sm text-foreground break-words leading-relaxed whitespace-pre-wrap"
            >
                {asset.data || "(empty)"}
            </p>
        {:else if category === "image"}
            {#if asset.storage === "file" && asset.path}
                <img
                    src="{API_BASE}/panel/{runId}/blob/{asset.path}"
                    alt="asset-{asset.seq}"
                    class="max-w-xs max-h-48 rounded-md border object-contain"
                    loading="lazy"
                />
            {:else}
                <span class="text-xs text-muted-foreground italic"
                    >[Image: inline data not renderable]</span
                >
            {/if}
        {:else if category === "json"}
            <button
                class="text-left w-full"
                onclick={() => (isJsonExpanded = !isJsonExpanded)}
            >
                {#if isJsonExpanded}
                    <pre
                        class="text-xs font-mono bg-muted/30 rounded-md p-2 overflow-auto max-h-64 text-foreground">{jsonPretty()}</pre>
                {:else}
                    <span
                        class="text-xs font-mono text-muted-foreground truncate block"
                    >
                        {asset.data?.slice(0, 120)}{(asset.data?.length ?? 0) >
                        120
                            ? "…"
                            : ""}
                    </span>
                {/if}
            </button>
        {:else}
            <span class="text-xs text-muted-foreground font-mono">
                [{asset.type}] {asset.storage === "file"
                    ? asset.path
                    : `${asset.data?.length ?? 0} bytes inline`}
            </span>
        {/if}
    </div>

    <!-- Seq number -->
    <div class="shrink-0 pt-0.5">
        <span
            class="text-[10px] font-mono text-muted-foreground/40 tabular-nums opacity-0 group-hover:opacity-100 transition-opacity"
        >
            #{asset.seq}
        </span>
    </div>
</div>
