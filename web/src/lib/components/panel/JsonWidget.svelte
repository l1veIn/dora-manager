<script lang="ts">
    import type { Asset } from "./types";
    import TextWidget from "./TextWidget.svelte";

    let { assets = [], isLive = false } = $props<{
        assets: Asset[];
        isLive?: boolean;
    }>();

    // In a full implementation, this could use svelte-json-tree or similar
    // For now, we reuse TextWidget but pretty-print the JSON if possible
    let formattedAssets = $derived(
        assets.map((a: Asset) => {
            try {
                if (a.data) {
                    const obj = JSON.parse(a.data);
                    return { ...a, data: JSON.stringify(obj, null, 2) };
                }
            } catch {
                // fallback to raw string
            }
            return a;
        }),
    );
</script>

<!-- Reuse TextWidget functionality but with pretty-printed JSON data -->
<TextWidget assets={formattedAssets} {isLive} />
