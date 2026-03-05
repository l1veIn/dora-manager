<script lang="ts">
    import type { Asset } from "./types";
    import ImageWidget from "./ImageWidget.svelte";
    import TextWidget from "./TextWidget.svelte";
    import JsonWidget from "./JsonWidget.svelte";
    import RawWidget from "./RawWidget.svelte";

    let {
        assets = [],
        runId,
        isLive = false,
    } = $props<{
        assets: Asset[];
        runId: string;
        isLive?: boolean;
    }>();

    // The dispatcher component needs to render differently based on if it's Live or Replay.
    // In Live mode, some widgets (like Image) only show the *latest* asset.
    // Text/JSON widgets might show a scrolling log.

    let latestAsset = $derived(
        assets.length > 0 ? assets[assets.length - 1] : null,
    );
</script>

{#if !latestAsset}
    <div
        class="flex items-center justify-center h-full w-full text-muted-foreground bg-muted/10"
    >
        Waiting for data...
    </div>
{:else if latestAsset.type.startsWith("image/")}
    <ImageWidget asset={latestAsset} {runId} />
{:else if latestAsset.type === "application/json"}
    <JsonWidget {assets} {isLive} />
{:else if latestAsset.type === "text/plain"}
    <TextWidget {assets} {isLive} />
{:else}
    <RawWidget asset={latestAsset} {runId} />
{/if}
