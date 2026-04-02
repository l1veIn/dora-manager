<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import WaveSurfer from "wavesurfer.js";
    import { Play, Pause } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";

    let { file, runId } = $props<{ file: string; runId: string }>();
    let audioUrl = $derived(`/api/runs/${runId}/artifacts/${file}`);

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

        wavesurfer.on("timeupdate", (currentTimePos) => {
            currentTime = formatTime(currentTimePos);
        });

        wavesurfer.on("play", () => (isPlaying = true));
        wavesurfer.on("pause", () => (isPlaying = false));
    });

    onDestroy(() => { if (wavesurfer) wavesurfer.destroy(); });

    function togglePlay() {
        if (wavesurfer && isLoaded) wavesurfer.playPause();
    }
</script>

<div class="flex items-center gap-3 w-full min-w-[280px] max-w-[400px]">
    <Button variant="secondary" size="icon" class="h-10 w-10 shrink-0 rounded-full bg-primary/10 hover:bg-primary/20 text-primary" onclick={togglePlay} disabled={!isLoaded}>
        {#if isPlaying}
            <Pause class="h-5 w-5 fill-current" />
        {:else}
            <Play class="h-5 w-5 fill-current ml-0.5" />
        {/if}
    </Button>
    <div class="flex-1 min-w-0">
        <div bind:this={container} class="w-full" style="opacity: {isLoaded ? 1 : 0.5}"></div>
        <div class="flex justify-between text-[10px] text-muted-foreground mt-1 font-mono tracking-tighter">
            <span>{currentTime}</span>
            <span>{duration}</span>
        </div>
    </div>
</div>
