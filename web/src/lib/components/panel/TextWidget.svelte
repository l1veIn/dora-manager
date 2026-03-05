<script lang="ts">
    import type { Asset } from "./types";
    import { onMount, tick } from "svelte";

    let { assets = [], isLive = false } = $props<{
        assets: Asset[];
        isLive?: boolean;
    }>();

    let scrollContainer = $state<HTMLElement | null>(null);
    let autoScroll = $state(true);

    // Auto-scroll logic when new assets arrive in live mode
    $effect(() => {
        if (assets && scrollContainer && autoScroll && isLive) {
            tick().then(() => {
                if (scrollContainer) {
                    scrollContainer.scrollTo({
                        top: scrollContainer.scrollHeight,
                        behavior: "smooth",
                    });
                }
            });
        }
    });

    function handleScroll(e: Event) {
        const target = e.target as HTMLElement;
        const isAtBottom =
            target.scrollHeight - target.scrollTop <= target.clientHeight + 10;
        autoScroll = isAtBottom;
    }
</script>

<div
    bind:this={scrollContainer}
    onscroll={handleScroll}
    class="w-full h-full overflow-auto bg-muted/10 p-2 font-mono text-xs whitespace-pre-wrap flex flex-col gap-1"
>
    {#each assets.slice(-100) as asset (asset.seq)}
        <div
            class="flex gap-2 group hover:bg-muted/50 rounded px-1 transition-colors"
        >
            <span
                class="text-muted-foreground/50 w-6 text-right shrink-0 select-none"
            >
                {asset.seq}
            </span>
            <span class="text-foreground break-all">
                {asset.data || "(Empty text)"}
            </span>
        </div>
    {/each}
</div>
