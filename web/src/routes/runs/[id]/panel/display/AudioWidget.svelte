<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import WaveSurfer from "wavesurfer.js";
    import { Download, Play, Pause } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";

    let { asset, runId } = $props<{
        asset: any;
        runId: string;
    }>();

    let audioUrl = $derived(`/api/runs/${runId}/panel/file/${asset.path}`);

    let container: HTMLDivElement;
    let wavesurfer: WaveSurfer | null = null;
    let isPlaying = $state(false);
    let duration = $state("0:00");
    let currentTime = $state("0:00");
    let isLoaded = $state(false);

    function formatTime(seconds: number) {
        if (!seconds || isNaN(seconds)) return "0:00";
        const mins = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${mins}:${secs.toString().padStart(2, "0")}`;
    }

    onMount(() => {
        if (!container) return;

        wavesurfer = WaveSurfer.create({
            container: container,
            waveColor: "rgba(100, 116, 139, 0.5)",
            progressColor: "rgb(59, 130, 246)",
            cursorColor: "rgb(59, 130, 246)",
            barWidth: 2,
            barGap: 2,
            barRadius: 2,
            height: 40,
            normalize: true,
            url: audioUrl,
        });

        wavesurfer.on("ready", () => {
            isLoaded = true;
            duration = formatTime(wavesurfer?.getDuration() || 0);
        });

        // Use timeupdate rather than audioprocess for newer versions
        wavesurfer.on("timeupdate", (currentTimePos) => {
            currentTime = formatTime(currentTimePos);
        });

        wavesurfer.on("play", () => (isPlaying = true));
        wavesurfer.on("pause", () => (isPlaying = false));
    });

    onDestroy(() => {
        if (wavesurfer) {
            wavesurfer.destroy();
        }
    });

    function togglePlay() {
        if (wavesurfer && isLoaded) {
            wavesurfer.playPause();
        }
    }
</script>

<div
    class="flex flex-col gap-2 w-full max-w-[450px] min-w-[350px] rounded-lg p-3 bg-card border shadow-sm group"
>
    <div class="flex items-center gap-3">
        <Button
            variant="secondary"
            size="icon"
            class="h-10 w-10 shrink-0 rounded-full"
            onclick={togglePlay}
            disabled={!isLoaded}
        >
            {#if isPlaying}
                <Pause class="h-5 w-5 fill-current" />
            {:else}
                <Play class="h-5 w-5 fill-current ml-0.5" />
            {/if}
        </Button>

        <div class="flex-1 min-w-0">
            <div
                bind:this={container}
                class="w-full"
                style="opacity: {isLoaded ? 1 : 0.5}"
            ></div>
            <div
                class="flex justify-between text-[10px] text-muted-foreground mt-1 font-mono"
            >
                <span>{currentTime}</span>
                <span>{duration}</span>
            </div>
        </div>
    </div>

    <div
        class="flex items-center justify-between px-1 mt-1 border-t pt-2 border-border/50"
    >
        <span
            class="text-[10px] text-muted-foreground font-mono truncate max-w-[200px]"
            title={asset.path}
        >
            {asset.path || "Audio Clip"}
        </span>
        <a
            href={audioUrl}
            download
            target="_blank"
            class="text-muted-foreground hover:text-foreground transition-colors opacity-0 group-hover:opacity-100"
            title="Download Audio"
        >
            <Download class="h-3 w-3" />
        </a>
    </div>
</div>
