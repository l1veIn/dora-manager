<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import Plyr from "plyr";
    import "plyr/dist/plyr.css";
    import { Download } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";

    let { asset, runId } = $props<{
        asset: any;
        runId: string;
    }>();

    let videoUrl = $derived(`/api/runs/${runId}/panel/file/${asset.path}`);
    let videoElement: HTMLVideoElement;
    let player: Plyr | null = null;

    onMount(() => {
        if (videoElement) {
            player = new Plyr(videoElement, {
                controls: [
                    "play-large",
                    "play",
                    "progress",
                    "current-time",
                    "mute",
                    "volume",
                    "fullscreen",
                ],
                ratio: "16:9",
            });
        }
    });

    onDestroy(() => {
        if (player) {
            player.destroy();
        }
    });
</script>

<div
    class="relative group inline-block w-full max-w-[650px] min-w-[350px] rounded-lg border bg-black shadow-md overflow-hidden"
    style="--plyr-color-main: hsl(var(--primary));"
>
    <!-- svelte-ignore a11y_media_has_caption -->
    <video
        bind:this={videoElement}
        src={videoUrl}
        crossorigin="anonymous"
        preload="metadata"
    ></video>

    <div
        class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10"
    >
        <a
            href={videoUrl}
            download
            target="_blank"
            onclick={(e) => e.stopPropagation()}
        >
            <Button
                variant="secondary"
                size="icon"
                class="h-7 w-7 rounded-sm shadow-md bg-background/80 backdrop-blur border-transparent text-foreground hover:bg-background/90"
                title="Download Video"
            >
                <Download class="h-3.5 w-3.5" />
            </Button>
        </a>
    </div>
</div>
