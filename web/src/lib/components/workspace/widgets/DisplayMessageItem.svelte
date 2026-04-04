<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { Download, ZoomIn } from "lucide-svelte";
    import "viewerjs/dist/viewer.css";
    import Viewer from "viewerjs";
    import MessageVideo from "./media/MessageVideo.svelte";
    import MessageAudio from "./media/MessageAudio.svelte";
    import MessageJson from "./media/MessageJson.svelte";

    let { runId, entry } = $props<{ runId: string, entry: any }>();
    
    let imgElement = $state<HTMLImageElement | null>(null);
    let viewer: Viewer | null = null;

    $effect(() => {
        if (entry.render === "image" && imgElement && !viewer) {
            viewer = new Viewer(imgElement, {
                inline: false,
                button: true,
                navbar: false,
                title: false,
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
        return () => {
            if (viewer) {
                viewer.destroy();
                viewer = null;
            }
        };
    });

    function showPreview() {
        if (viewer) {
            viewer.show();
        }
    }
</script>

<div class="flex flex-col w-full group">
    <!-- Bubble Meta Info -->
    <div class="flex items-center gap-1.5 px-2 mb-1">
        <span class="text-[10px] font-mono font-medium tracking-tight text-primary/70">{entry.node_id}</span>
        <span class="text-[9px] text-muted-foreground/50">{new Date((entry.created_at || 0) * 1000).toLocaleTimeString()}</span>
        {#if entry.kind === 'file'}
            <span class="text-[9px] text-muted-foreground/40 font-mono" title={entry.file}>
                (📁 {entry.file})
            </span>
        {/if}
    </div>

    <!-- Bubble Content -->
    <div class="bg-card border shadow-sm rounded-2xl rounded-tl-sm px-3 py-2 max-w-[85%] self-start w-fit text-sm relative">
        <!-- Render Badge -->
        {#if entry.render}
            <div class="absolute -right-1 -bottom-2 bg-muted/60 text-[8px] uppercase tracking-wider px-1.5 py-0.5 rounded shadow-sm text-muted-foreground border">
                {entry.render}
            </div>
        {/if}

        {#if entry.render === "image"}
            <div class="relative group inline-block max-w-full">
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <img
                    bind:this={imgElement}
                    src={`/api/runs/${runId}/artifacts/${entry.file}`}
                    alt={entry.label}
                    class="max-w-full rounded border cursor-zoom-in hover:opacity-90 transition-opacity bg-black/5 object-contain"
                    style="max-height: 250px;"
                    onclick={showPreview}
                />
                
                <!-- Hover actions -->
                <div class="absolute top-2 right-2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <Button variant="secondary" size="icon" class="h-7 w-7 rounded-sm shadow-md bg-background/80 backdrop-blur" onclick={showPreview} title="Preview">
                        <ZoomIn class="h-3.5 w-3.5" />
                    </Button>
                    <a href={`/api/runs/${runId}/artifacts/${entry.file}`} download target="_blank" onclick={(e) => e.stopPropagation()}>
                        <Button variant="secondary" size="icon" class="h-7 w-7 rounded-sm shadow-md bg-background/80 backdrop-blur" title="Download">
                            <Download class="h-3.5 w-3.5" />
                        </Button>
                    </a>
                </div>
            </div>
        {:else if entry.render === "video"}
            <MessageVideo file={entry.file} {runId} />
        {:else if entry.render === "audio"}
            <MessageAudio file={entry.file} {runId} />
        {:else if entry.render === "json"}
            <MessageJson content={entry.content} />
        {:else if entry.render === "markdown"}
            <div class="prose prose-sm dark:prose-invert max-w-full prose-p:leading-relaxed break-words">{entry.content}</div>
        {:else}
            <!-- text fallthrough -->
            <div class="font-mono text-[11px] whitespace-pre-wrap break-words max-h-64 overflow-y-auto leading-relaxed">{entry.content}</div>
        {/if}
    </div>
</div>
