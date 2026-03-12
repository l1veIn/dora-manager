<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { Download, ZoomIn } from "lucide-svelte";
    import { onMount, onDestroy } from "svelte";
    import "viewerjs/dist/viewer.css";
    import Viewer from "viewerjs";

    let { asset, runId } = $props<{
        asset: any;
        runId: string;
    }>();

    let imgUrl = $derived(`/api/runs/${runId}/panel/file/${asset.path}`);
    let imgElement: HTMLImageElement;
    let viewer: Viewer | null = null;

    onMount(() => {
        if (imgElement) {
            viewer = new Viewer(imgElement, {
                inline: false,
                button: true, // Show top right close button
                navbar: false, // Don't show bottom thumbnails
                title: false, // Don't show image title
                toolbar: {
                    zoomIn: 1,
                    zoomOut: 1,
                    oneToOne: 1,
                    reset: 1,
                    prev: 0,
                    play: 0,
                    next: 0,
                    rotateLeft: 1,
                    rotateRight: 1,
                    flipHorizontal: 1,
                    flipVertical: 1,
                },
                tooltip: true,
                movable: true,
                zoomable: true,
                rotatable: true,
                scalable: true,
                transition: true,
                fullscreen: true,
                keyboard: true,
            });
        }
    });

    onDestroy(() => {
        if (viewer) {
            viewer.destroy();
        }
    });

    function showPreview() {
        if (viewer) {
            viewer.show();
        }
    }
</script>

<div class="relative group inline-block max-w-full">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <img
        bind:this={imgElement}
        src={imgUrl}
        alt="Panel Output"
        class="max-w-full rounded border cursor-zoom-in hover:opacity-90 transition-opacity bg-black/5 object-contain"
        style="max-height: 250px;"
        onclick={showPreview}
    />

    <!-- Hover actions -->
    <div
        class="absolute top-2 right-2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
    >
        <Button
            variant="secondary"
            size="icon"
            class="h-7 w-7 rounded-sm shadow-md bg-background/80 backdrop-blur"
            onclick={showPreview}
            title="Preview"
        >
            <ZoomIn class="h-3.5 w-3.5" />
        </Button>
        <a
            href={imgUrl}
            download
            target="_blank"
            onclick={(e) => e.stopPropagation()}
        >
            <Button
                variant="secondary"
                size="icon"
                class="h-7 w-7 rounded-sm shadow-md bg-background/80 backdrop-blur"
                title="Download"
            >
                <Download class="h-3.5 w-3.5" />
            </Button>
        </a>
    </div>
</div>
