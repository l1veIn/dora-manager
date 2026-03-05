<script lang="ts">
    import type { Asset } from "./types";
    import { API_BASE } from "$lib/api";

    let { asset, runId } = $props<{
        asset: Asset;
        runId: string;
    }>();

    // Derived image source based on storage type
    let src = $derived(() => {
        if (asset.storage === "file" && asset.path) {
            return `${API_BASE}/panel/${runId}/file/${asset.path}`;
        } else if (asset.storage === "inline" && asset.data) {
            // Very small images might be inline base64, though unlikely for dm-panel
            return `data:${asset.type};base64,${asset.data}`;
        }
        return "";
    });
</script>

<div
    class="relative w-full h-full flex items-center justify-center bg-black overflow-hidden group"
>
    {#if src()}
        <img
            src={src()}
            alt={`Frame ${asset.seq}`}
            class="max-w-full max-h-full object-contain"
        />

        <!-- Overlay HUD -->
        <div
            class="absolute bottom-0 inset-x-0 p-2 bg-gradient-to-t from-black/80 to-transparent flex items-center justify-between text-[10px] font-mono text-white opacity-0 group-hover:opacity-100 transition-opacity"
        >
            <span class="bg-black/50 px-1.5 py-0.5 rounded">#{asset.seq}</span>
            <span class="opacity-70"
                >{new Date(asset.timestamp)
                    .toISOString()
                    .split("T")[1]
                    .replace("Z", "")}</span
            >
        </div>
    {:else}
        <div class="text-xs text-muted-foreground">Invalid Image Asset</div>
    {/if}
</div>
