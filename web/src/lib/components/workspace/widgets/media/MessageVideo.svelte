<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import Plyr from "plyr";
    import "plyr/dist/plyr.css";

    let { file, runId } = $props<{ file: string; runId: string }>();
    let videoUrl = $derived(`/api/runs/${runId}/artifacts/${file}`);
    let videoElement: HTMLVideoElement;
    let player: Plyr | null = null;

    onMount(() => {
        if (videoElement) {
            player = new Plyr(videoElement, {
                controls: ["play-large", "play", "progress", "current-time", "mute", "volume", "fullscreen"],
                ratio: "16:9",
            });
        }
    });

    onDestroy(() => { if (player) player.destroy(); });
</script>

<div class="w-full max-w-[500px] min-w-[300px] rounded overflow-hidden" style="--plyr-color-main: hsl(var(--primary));">
    <!-- svelte-ignore a11y_media_has_caption -->
    <video bind:this={videoElement} src={videoUrl} crossorigin="anonymous" preload="metadata"></video>
</div>
