<script lang="ts">
    import type { Asset } from "./types";
    import { API_BASE } from "$lib/api";
    import { Download, FileQuestion } from "lucide-svelte";

    let { asset, runId } = $props<{
        asset: Asset;
        runId: string;
    }>();

    function assetUrl(path: string) {
        return `${API_BASE}/panel/${runId}/file/${path}`;
    }
</script>

<div
    class="flex flex-col items-center justify-center p-6 h-full w-full bg-muted/5 text-center gap-4"
>
    <div class="p-3 bg-muted rounded-full">
        <FileQuestion class="size-8 text-muted-foreground" />
    </div>

    <div>
        <h3 class="font-medium text-sm">Binary Data</h3>
        <p class="text-xs text-muted-foreground mt-1 font-mono">{asset.type}</p>
        <p class="text-xs text-muted-foreground mt-0.5">Seq: {asset.seq}</p>
    </div>

    {#if asset.storage === "file" && asset.path}
        <a
            href={assetUrl(asset.path)}
            download
            target="_blank"
            rel="noopener noreferrer"
            class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground h-9 px-4 py-2 mt-2 gap-2"
        >
            <Download class="size-4" />
            Download File
        </a>
    {:else if asset.data}
        <div
            class="max-w-full overflow-hidden text-ellipsis px-4 py-2 bg-muted/50 rounded border text-xs font-mono text-muted-foreground/80 break-all"
        >
            {asset.data.substring(0, 200)}{asset.data.length > 200 ? "..." : ""}
        </div>
    {/if}
</div>
