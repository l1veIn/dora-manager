<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { Download, ZoomIn } from "lucide-svelte";
    import "viewerjs/dist/viewer.css";
    import Viewer from "viewerjs";
    import MessageVideo from "./media/MessageVideo.svelte";
    import MessageAudio from "./media/MessageAudio.svelte";
    import MessageJson from "./media/MessageJson.svelte";
    import UserInputMessageItem from "./UserInputMessageItem.svelte";
    import WidgetRegistrationItem from "./WidgetRegistrationItem.svelte";

    let { runId, entry } = $props<{ runId: string, entry: any }>();

    let imgElement = $state<HTMLImageElement | null>(null);
    let viewer: Viewer | null = null;

    const payload = $derived(entry.payload ?? {});
    const file = $derived(payload.file ?? null);
    const content = $derived(payload.content ?? "");
    const label = $derived(payload.label ?? entry.from);
    const kind = $derived(payload.kind ?? (file ? "file" : "inline"));
    const render = $derived(entry.tag ?? "text");
    const isUserInputMessage = $derived(entry.from === "web" && render === "input");
    const isWidgetRegistration = $derived(render === "widgets");
    const isKnownRender = $derived(
        render === "text" ||
            render === "image" ||
            render === "video" ||
            render === "audio" ||
            render === "json" ||
            render === "markdown" ||
            render === "input" ||
            render === "widgets",
    );

    $effect(() => {
        if (render === "image" && imgElement && !viewer) {
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

{#if isUserInputMessage}
    <UserInputMessageItem {entry} />
{:else if isWidgetRegistration}
    <WidgetRegistrationItem {entry} />
{:else}
<div class="flex flex-col w-full group">
    <div class="flex items-center gap-1.5 px-2 mb-1">
        <span class="text-[10px] font-mono font-medium tracking-tight text-primary/70">{entry.from}</span>
        <span class="text-[9px] text-muted-foreground/50">{new Date((entry.timestamp || 0) * 1000).toLocaleTimeString()}</span>
        {#if kind === 'file' && file}
            <span class="text-[9px] text-muted-foreground/40 font-mono" title={file}>
                (📁 {file})
            </span>
        {/if}
    </div>

    <div class="bg-card border shadow-sm rounded-2xl rounded-tl-sm px-3 py-2 max-w-[85%] self-start w-fit text-sm relative">
        {#if render}
            <div class="absolute -right-1 -bottom-2 bg-muted/60 text-[8px] uppercase tracking-wider px-1.5 py-0.5 rounded shadow-sm text-muted-foreground border">
                {render}
            </div>
        {/if}

        {#if render === "image" && file}
            <div class="relative group inline-block max-w-full">
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <img
                    bind:this={imgElement}
                    src={`/api/runs/${runId}/artifacts/${file}`}
                    alt={label}
                    class="max-w-full rounded border cursor-zoom-in hover:opacity-90 transition-opacity bg-black/5 object-contain"
                    style="max-height: 250px;"
                    onclick={showPreview}
                />
                <div class="absolute top-2 right-2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <Button variant="secondary" size="icon" class="h-7 w-7 rounded-sm shadow-md bg-background/80 backdrop-blur" onclick={showPreview} title="Preview">
                        <ZoomIn class="h-3.5 w-3.5" />
                    </Button>
                    <a href={`/api/runs/${runId}/artifacts/${file}`} download target="_blank" onclick={(e) => e.stopPropagation()}>
                        <Button variant="secondary" size="icon" class="h-7 w-7 rounded-sm shadow-md bg-background/80 backdrop-blur" title="Download">
                            <Download class="h-3.5 w-3.5" />
                        </Button>
                    </a>
                </div>
            </div>
        {:else if render === "video" && file}
            <MessageVideo {file} {runId} />
        {:else if render === "audio" && file}
            <MessageAudio {file} {runId} />
        {:else if render === "json"}
            <MessageJson content={content} />
        {:else if render === "markdown"}
            <div class="prose prose-sm dark:prose-invert max-w-full prose-p:leading-relaxed break-words">{content}</div>
        {:else if render === "text"}
            <div class="font-mono text-[11px] whitespace-pre-wrap break-words max-h-64 overflow-y-auto leading-relaxed">{content}</div>
        {:else if !isKnownRender}
            <div class="space-y-2">
                <div class="text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                    Default JSON View
                </div>
                <MessageJson content={payload} />
            </div>
        {:else}
            <MessageJson content={payload} />
        {/if}
    </div>
</div>
{/if}
