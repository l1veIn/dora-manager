<script lang="ts">
    import type { Asset } from "./types";
    import { Slider } from "$lib/components/ui/slider/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Play, Pause, FastForward, Rewind } from "lucide-svelte";
    import { onMount } from "svelte";

    let { assets = [] } = $props<{
        assets: Asset[];
    }>();

    // Timeline state
    let isPlaying = $state(false);
    let playbackSpeed = $state(1);
    let currentIndex = $state(0);
    // Bind slider value
    let sliderValue = $state(0);

    // Update currentIndex based on slider drag
    $effect(() => {
        if (sliderValue !== undefined) {
            currentIndex = sliderValue;
            // Filter logic in parent would apply here or emit event
            // For now, since it relies on parent's filter:
            // The simplest approach is to dispatch or bind the current "active limit/range"
        }
    });

    // Auto update slider when playing
    $effect(() => {
        if (assets.length > 0 && currentIndex !== sliderValue) {
            sliderValue = currentIndex;
        }
    });

    let playInterval: ReturnType<typeof setInterval> | null = null;

    function togglePlayback() {
        isPlaying = !isPlaying;
        if (isPlaying) {
            if (currentIndex >= assets.length - 1) {
                currentIndex = 0;
            }
            playInterval = setInterval(
                () => {
                    if (currentIndex < assets.length - 1) {
                        currentIndex++;
                    } else {
                        pausePlayback();
                    }
                },
                1000 / (30 * playbackSpeed),
            ); // Arbitrary base fps
        } else {
            pausePlayback();
        }
    }

    function pausePlayback() {
        isPlaying = false;
        if (playInterval) clearInterval(playInterval);
    }

    function formatTime(timestamp: string) {
        if (!timestamp) return "00:00.000";
        const d = new Date(timestamp);
        return d.toISOString().split("T")[1].replace("Z", "");
    }

    onMount(() => {
        return () => pausePlayback();
    });
</script>

<div class="flex items-center gap-4 w-full px-2">
    <!-- Playback Controls -->
    <div
        class="flex items-center gap-1 shrink-0 bg-muted/50 p-1 rounded-md border"
    >
        <Button
            variant="ghost"
            size="icon"
            class="size-8 text-muted-foreground hover:text-foreground"
            onclick={() => (currentIndex = Math.max(0, currentIndex - 10))}
        >
            <Rewind class="size-4" />
        </Button>
        <Button
            variant="secondary"
            size="icon"
            class="size-8"
            onclick={togglePlayback}
        >
            {#if isPlaying}
                <Pause class="size-4" />
            {:else}
                <Play class="size-4 ml-0.5" />
            {/if}
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="size-8 text-muted-foreground hover:text-foreground"
            onclick={() =>
                (currentIndex = Math.min(assets.length - 1, currentIndex + 10))}
        >
            <FastForward class="size-4" />
        </Button>

        <div class="h-4 w-px bg-border mx-1"></div>

        <button
            class="text-[10px] font-mono font-medium px-2 rounded hover:bg-muted transition-colors opacity-70"
            onclick={() =>
                (playbackSpeed =
                    playbackSpeed === 1 ? 2 : playbackSpeed === 2 ? 0.5 : 1)}
        >
            {playbackSpeed}x
        </button>
    </div>

    <!-- Timeline Slider -->
    <div class="flex-1 flex flex-col gap-1.5 px-2 justify-center pt-2">
        <div class="w-full">
            <Slider
                type="single"
                min={0}
                max={Math.max(0, assets.length - 1)}
                step={1}
                bind:value={sliderValue}
                class="hover:cursor-pointer [&>[role=slider]]:size-4"
            />
        </div>
        <div
            class="flex justify-between items-center text-[10px] font-mono text-muted-foreground/70 px-1"
        >
            <span
                >{assets.length > 0
                    ? formatTime(assets[0].timestamp)
                    : "--:--.---"}</span
            >
            <span
                class="font-bold text-foreground mx-auto bg-muted px-2 py-0.5 rounded border shadow-sm z-10 -mt-8"
            >
                {assets.length > 0
                    ? formatTime(assets[currentIndex]?.timestamp)
                    : "--:--.---"}
            </span>
            <span
                >{assets.length > 0
                    ? formatTime(assets[assets.length - 1].timestamp)
                    : "--:--.---"}</span
            >
        </div>
    </div>
</div>
