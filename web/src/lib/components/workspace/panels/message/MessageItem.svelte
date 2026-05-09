<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { AlertTriangle, CheckCircle2, Download, LoaderCircle, XCircle, ZoomIn } from "lucide-svelte";
    import { marked } from "marked";
    import DOMPurify from "dompurify";
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
    const embed = $derived(payload.embed ?? null);
    const hasEmbed = $derived(isNonEmptyObject(embed));
    const embedAccentColor = $derived(resolveEmbedColor(embed?.color));
    const embedMarkdownBody = $derived(renderMarkdown(embed?.body_formatted ?? ""));
    const embedTimestampLabel = $derived(formatEmbedTimestamp(entry.timestamp, embed?.timestamp));
    const embedContainerClass = $derived(embedAlignmentClass(embed?.side));
    const embedCardClass = $derived(embedWidthClass(embed?.width, embed?.side));
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

    function isNonEmptyObject(value: unknown) {
        return !!value && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length > 0;
    }

    function renderMarkdown(value: unknown) {
        if (typeof value !== "string" || !value.trim()) return "";
        return DOMPurify.sanitize(marked.parse(value) as string);
    }

    function resolveEmbedColor(value: unknown) {
        const colors: Record<string, string> = {
            green: "#22c55e",
            red: "#ef4444",
            yellow: "#eab308",
            blue: "#3b82f6",
            purple: "#a855f7",
            gray: "#6b7280",
            orange: "#f97316",
        };

        if (typeof value === "number" && Number.isFinite(value)) {
            return `#${Math.max(0, Math.min(0xffffff, value)).toString(16).padStart(6, "0")}`;
        }

        if (typeof value !== "string") return null;
        const normalized = value.trim().toLowerCase();
        if (colors[normalized]) return colors[normalized];
        if (/^#[0-9a-f]{6}$/i.test(normalized)) return normalized;
        if (/^0x[0-9a-f]{1,6}$/i.test(normalized)) {
            return `#${parseInt(normalized.slice(2), 16).toString(16).padStart(6, "0")}`;
        }
        if (/^[0-9a-f]{6}$/i.test(normalized)) return `#${normalized}`;
        return null;
    }

    function toTimestampDate(value: unknown) {
        const numeric = Number(value);
        if (!Number.isFinite(numeric) || numeric <= 0) return null;
        return new Date(numeric > 9999999999 ? numeric : numeric * 1000);
    }

    function formatEmbedTimestamp(value: unknown, mode: unknown) {
        if (mode === "hidden") return "";
        const date = toTimestampDate(value);
        if (!date) return "";
        if (mode === "absolute") return date.toLocaleString();
        return date.toLocaleTimeString();
    }

    function embedAlignmentClass(side: unknown) {
        if (side === "right") return "items-end";
        if (side === "center") return "items-center";
        return "items-start";
    }

    function embedWidthClass(width: unknown, side: unknown) {
        const base = "bg-card border shadow-sm rounded-xl px-3 py-2 text-sm relative overflow-hidden";
        const sizing = width === "full" ? "w-full" : "w-fit max-w-[85%]";
        const sideStyle = side === "right" ? "rounded-tr-sm bg-primary/5" : side === "center" ? "" : "rounded-tl-sm";
        return `${base} ${sizing} ${sideStyle}`;
    }

    function mediaStyle(media: any) {
        const width = Number(media?.width);
        const height = Number(media?.height);
        const maxWidth = Number.isFinite(width) && width > 0 ? `max-width: ${Math.min(width, 720)}px;` : "";
        const aspectRatio = Number.isFinite(width) && width > 0 && Number.isFinite(height) && height > 0 ? `aspect-ratio: ${width} / ${height};` : "";
        return `${maxWidth} ${aspectRatio}`;
    }

    function progressPercent(value: unknown) {
        const numeric = Number(value);
        if (!Number.isFinite(numeric)) return 0;
        return Math.max(0, Math.min(100, numeric * 100));
    }
</script>

{#if isUserInputMessage}
    <UserInputMessageItem {entry} />
{:else if isWidgetRegistration}
    <WidgetRegistrationItem {entry} />
{:else if hasEmbed}
<div class={`flex flex-col w-full group ${embedContainerClass}`}>
    <div class={embedCardClass} style={embedAccentColor ? `border-left: 4px solid ${embedAccentColor} !important;` : ""}>
        {#if embed.thumbnail?.url}
            <img
                src={embed.thumbnail.url}
                alt={embed.thumbnail.alt ?? ""}
                class="absolute top-3 right-3 h-14 w-14 rounded-md border bg-muted object-cover"
                style={mediaStyle(embed.thumbnail)}
            />
        {/if}

        <div class={embed.thumbnail?.url ? "space-y-2 pr-16" : "space-y-2"}>
            {#if embed.author}
                <div class="flex items-center justify-between gap-3 text-xs text-muted-foreground">
                    {#if embed.author.url}
                        <a href={embed.author.url} class="inline-flex min-w-0 items-center gap-1.5 font-medium text-foreground/80 hover:underline" target={String(embed.author.url).startsWith("http") ? "_blank" : undefined} rel={String(embed.author.url).startsWith("http") ? "noreferrer" : undefined}>
                            {#if embed.author.icon}<span class="shrink-0">{embed.author.icon}</span>{/if}
                            <span class="truncate">{embed.author.name ?? entry.from}</span>
                        </a>
                    {:else}
                        <div class="inline-flex min-w-0 items-center gap-1.5 font-medium text-foreground/80">
                            {#if embed.author.icon}<span class="shrink-0">{embed.author.icon}</span>{/if}
                            <span class="truncate">{embed.author.name ?? entry.from}</span>
                        </div>
                    {/if}
                    {#if embedTimestampLabel}
                        <span class="shrink-0 text-[10px] text-muted-foreground/60">{embedTimestampLabel}</span>
                    {/if}
                </div>
            {:else if embedTimestampLabel}
                <div class="flex items-center justify-between gap-3 text-xs text-muted-foreground">
                    <span class="font-mono text-[10px] font-medium text-primary/70">{entry.from}</span>
                    <span class="shrink-0 text-[10px] text-muted-foreground/60">{embedTimestampLabel}</span>
                </div>
            {/if}

            {#if embed.title}
                {#if embed.title_url}
                    <a href={embed.title_url} class="block text-sm font-semibold leading-snug text-foreground hover:underline" target={String(embed.title_url).startsWith("http") ? "_blank" : undefined} rel={String(embed.title_url).startsWith("http") ? "noreferrer" : undefined}>
                        {embed.title}
                    </a>
                {:else}
                    <div class="text-sm font-semibold leading-snug text-foreground">{embed.title}</div>
                {/if}
            {/if}

            {#if embed.body_formatted && embed.body_format === "markdown"}
                <div class="prose prose-sm dark:prose-invert max-w-none break-words prose-p:my-1 prose-p:leading-relaxed prose-pre:my-2 prose-code:text-[0.85em]">
                    {@html embedMarkdownBody}
                </div>
            {:else if embed.body}
                <div class="whitespace-pre-wrap break-words text-sm leading-relaxed text-foreground/90">{embed.body}</div>
            {/if}

            {#if Array.isArray(embed.fields) && embed.fields.length}
                <div class="grid grid-cols-1 gap-2 pt-1 sm:grid-cols-2">
                    {#each embed.fields as field}
                        <div class={field?.inline ? "min-w-0 rounded-md border bg-muted/20 px-2 py-1.5" : "min-w-0 rounded-md border bg-muted/20 px-2 py-1.5 sm:col-span-2"}>
                            <div class="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{field?.name}</div>
                            <div class="mt-0.5 whitespace-pre-wrap break-words text-xs leading-relaxed text-foreground/90">{field?.value}</div>
                        </div>
                    {/each}
                </div>
            {/if}

            {#if embed.media?.url}
                <div class="pt-1">
                    {#if embed.media.type === "video"}
                        <!-- svelte-ignore a11y_media_has_caption -->
                        <video src={embed.media.url} controls class="max-h-72 w-full rounded-md border bg-black object-contain" style={mediaStyle(embed.media)} aria-label={embed.media.alt ?? embed.media.caption ?? "Video attachment"}></video>
                    {:else if embed.media.type === "audio"}
                        <audio src={embed.media.url} controls class="w-full" aria-label={embed.media.alt ?? embed.media.caption ?? "Audio attachment"}></audio>
                    {:else}
                        <img src={embed.media.url} alt={embed.media.alt ?? embed.media.caption ?? ""} class="max-h-72 max-w-full rounded-md border bg-muted object-contain" style={mediaStyle(embed.media)} />
                    {/if}
                    {#if embed.media.caption}
                        <div class="mt-1 text-[11px] text-muted-foreground">{embed.media.caption}</div>
                    {/if}
                </div>
            {/if}

            {#if embed.footer}
                <div class="flex items-center gap-1.5 pt-1 text-[11px] text-muted-foreground">
                    {#if embed.footer.icon}<span>{embed.footer.icon}</span>{/if}
                    {#if embed.footer.text}<span>{embed.footer.text}</span>{/if}
                </div>
            {/if}

            {#if Array.isArray(embed.actions) && embed.actions.length}
                <div class="flex flex-wrap gap-2 pt-1">
                    {#each embed.actions as action}
                        {#if action?.url}
                            <a
                                href={action.url}
                                target={String(action.url).startsWith("http") ? "_blank" : undefined}
                                rel={String(action.url).startsWith("http") ? "noreferrer" : undefined}
                                class={action.style === "button" ? "inline-flex h-8 items-center justify-center rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-sm hover:bg-primary/90" : "inline-flex h-8 items-center justify-center rounded-md border bg-background px-3 text-xs font-medium hover:bg-muted"}
                            >
                                {action.label ?? "Open"}
                            </a>
                        {/if}
                    {/each}
                </div>
            {/if}

            {#if embed.status}
                <div class="pt-1">
                    {#if (embed.status === "pending" || embed.status === "running") && embed.progress !== undefined}
                        <div class="flex items-center gap-2 text-[11px] text-muted-foreground">
                            <LoaderCircle class="h-3.5 w-3.5 animate-spin" />
                            <div class="h-1.5 min-w-28 flex-1 overflow-hidden rounded-full bg-muted">
                                <div class="h-full rounded-full bg-primary transition-all" style={`width: ${progressPercent(embed.progress)}%;`}></div>
                            </div>
                            <span class="font-mono">{Math.round(progressPercent(embed.progress))}%</span>
                        </div>
                    {:else if embed.status === "success"}
                        <div class="flex items-center gap-1.5 text-xs font-medium text-green-600 dark:text-green-400"><CheckCircle2 class="h-3.5 w-3.5" /> Success</div>
                    {:else if embed.status === "warning"}
                        <div class="flex items-center gap-1.5 text-xs font-medium text-yellow-600 dark:text-yellow-400"><AlertTriangle class="h-3.5 w-3.5" /> Warning</div>
                    {:else if embed.status === "error"}
                        <div class="flex items-center gap-1.5 text-xs font-medium text-red-600 dark:text-red-400"><XCircle class="h-3.5 w-3.5" /> Error</div>
                    {:else}
                        <div class="text-xs font-medium text-muted-foreground capitalize">{embed.status}</div>
                    {/if}
                </div>
            {/if}
        </div>
    </div>
</div>
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
