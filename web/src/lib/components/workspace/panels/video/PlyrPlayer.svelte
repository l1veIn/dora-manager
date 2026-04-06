<script lang="ts">
    import { browser } from "$app/environment";
    import { onMount } from "svelte";
    import Hls from "hls.js";
    import Plyr from "plyr";
    import "plyr/dist/plyr.css";

    type SourceType = "auto" | "hls" | "video" | "audio";

    let {
        src,
        type = "auto",
        poster = "",
        autoplay = false,
        muted = true,
    }: {
        src: string;
        type?: SourceType;
        poster?: string;
        autoplay?: boolean;
        muted?: boolean;
    } = $props();

    let mediaEl = $state<HTMLVideoElement | HTMLAudioElement | null>(null);
    let error = $state<string | null>(null);

    let player: Plyr | null = null;
    let hls: Hls | null = null;

    const resolvedType = $derived<Exclude<SourceType, "auto">>(
        type === "auto"
            ? /\.m3u8($|\?)/i.test(src)
                ? "hls"
                : /\.mp3($|\?)/i.test(src) || /\.wav($|\?)/i.test(src) || /\.ogg($|\?)/i.test(src)
                  ? "audio"
                  : "video"
            : type,
    );

    function cleanup() {
        if (player) {
            player.destroy();
            player = null;
        }
        if (hls) {
            hls.destroy();
            hls = null;
        }
        if (mediaEl) {
            mediaEl.pause?.();
            mediaEl.removeAttribute("src");
            mediaEl.load?.();
        }
    }

    function setup() {
        cleanup();

        if (!browser || !mediaEl || !src) {
            return;
        }

        error = null;

        if (mediaEl instanceof HTMLVideoElement) {
            mediaEl.poster = poster || "";
            mediaEl.playsInline = true;
        }
        mediaEl.autoplay = autoplay;
        mediaEl.muted = muted;
        mediaEl.controls = true;

        if (resolvedType === "hls" && mediaEl instanceof HTMLVideoElement && Hls.isSupported()) {
            hls = new Hls({
                enableWorker: true,
                manifestLoadingMaxRetry: 0,
                levelLoadingMaxRetry: 0,
                fragLoadingMaxRetry: 0,
            });
            hls.on(Hls.Events.ERROR, (_event, data) => {
                if (data.fatal) {
                    error = data.details || "Failed to load HLS stream.";
                }
            });
            hls.loadSource(src);
            hls.attachMedia(mediaEl);
        } else {
            mediaEl.src = src;
        }

        player = new Plyr(mediaEl, {
            autoplay,
            muted,
            controls:
                resolvedType === "audio"
                    ? ["play", "progress", "current-time", "mute", "volume"]
                    : [
                          "play-large",
                          "play",
                          "progress",
                          "current-time",
                          "mute",
                          "volume",
                          "settings",
                          "pip",
                          "airplay",
                          "fullscreen",
                      ],
        });

        mediaEl.addEventListener(
            "error",
            () => {
                error = `Failed to load media source.`;
            },
            { once: true },
        );
    }

    onMount(() => {
        setup();
        return cleanup;
    });
</script>

{#if resolvedType === "audio"}
    <div class="flex h-full w-full items-center justify-center bg-background p-4">
        <audio bind:this={mediaEl} class="w-full max-w-xl" preload="metadata"></audio>
    </div>
{:else}
    <div class="h-full w-full bg-black">
        <video bind:this={mediaEl} class="h-full w-full" preload="metadata"></video>
    </div>
{/if}

{#if error}
    <div class="pointer-events-none absolute inset-x-4 bottom-4 rounded-md bg-background/90 px-3 py-2 text-xs text-destructive shadow-sm">
        {error}
    </div>
{/if}
