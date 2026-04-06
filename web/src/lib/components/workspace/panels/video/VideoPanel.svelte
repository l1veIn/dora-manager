<script lang="ts">
    import { browser } from "$app/environment";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import type { PanelRendererProps } from "../types";
    import PlyrPlayer from "./PlyrPlayer.svelte";

    type SourceType = "auto" | "hls" | "video" | "audio";
    type PlayerMode = "manual" | "message";

    type StreamSource = {
        id: string;
        nodeId: string;
        title: string;
        label: string;
        url: string;
        type: SourceType;
        poster?: string;
    };

    let { item, context, onConfigChange }: PanelRendererProps = $props();

    function ensureConfig() {
        if (!item.config) item.config = {};
        if (!item.config.mode) item.config.mode = "manual";
        if (!item.config.nodeId) item.config.nodeId = "*";
        if (typeof item.config.selectedSourceId !== "string") item.config.selectedSourceId = "";
        if (typeof item.config.src !== "string") item.config.src = "";
        if (!item.config.sourceType) item.config.sourceType = "hls";
        if (typeof item.config.autoplay !== "boolean") item.config.autoplay = false;
        if (typeof item.config.muted !== "boolean") item.config.muted = true;
        if (typeof item.config.poster !== "string") item.config.poster = "";
    }

    ensureConfig();

    function updateConfig() {
        onConfigChange?.();
    }

    function inferSourceType(raw: any): SourceType {
        const value = String(raw ?? "").toLowerCase();
        if (
            value === "hls" ||
            value.includes("mpegurl") ||
            value.endsWith(".m3u8")
        ) {
            return "hls";
        }
        if (
            value === "audio" ||
            value.startsWith("audio/") ||
            value.endsWith(".mp3") ||
            value.endsWith(".wav") ||
            value.endsWith(".ogg")
        ) {
            return "audio";
        }
        if (value === "video" || value.startsWith("video/")) {
            return "video";
        }
        return "video";
    }

    function inferLegacyHlsUrl(path: string | undefined): string {
        if (!browser || !path) return "";
        return `http://${window.location.hostname}:8888/${path}/index.m3u8`;
    }

    function extractSources(snapshot: any): StreamSource[] {
        const payload = snapshot?.payload ?? {};
        const title = payload.label || payload.title || snapshot.node_id;
        const nodeId = snapshot.node_id;
        const entries: StreamSource[] = [];

        if (Array.isArray(payload.sources)) {
            payload.sources.forEach((entry: any, index: number) => {
                const url = entry?.url || entry?.src;
                if (!url) return;
                entries.push({
                    id: `${nodeId}:${snapshot.seq}:source:${entry.id ?? index}`,
                    nodeId,
                    title,
                    label: entry?.label || entry?.name || entry?.id || `Source ${index + 1}`,
                    url,
                    type: inferSourceType(entry?.type || entry?.mime || url),
                    poster: entry?.poster || payload.poster,
                });
            });
        }

        if (payload.url || payload.src) {
            const url = payload.url || payload.src;
            entries.push({
                id: `${nodeId}:${snapshot.seq}:url`,
                nodeId,
                title,
                label: payload.source_label || "Primary Source",
                url,
                type: inferSourceType(payload.type || payload.mime || url),
                poster: payload.poster,
            });
        }

        if (payload.hls_url) {
            entries.push({
                id: `${nodeId}:${snapshot.seq}:hls`,
                nodeId,
                title,
                label: "HLS",
                url: payload.hls_url,
                type: "hls",
                poster: payload.poster,
            });
        }

        if (payload.viewer?.hls_url) {
            entries.push({
                id: `${nodeId}:${snapshot.seq}:viewer-hls`,
                nodeId,
                title,
                label: "Viewer HLS",
                url: payload.viewer.hls_url,
                type: "hls",
                poster: payload.poster,
            });
        }

        if (entries.length === 0 && payload.path) {
            const legacyUrl = inferLegacyHlsUrl(payload.path);
            if (legacyUrl) {
                entries.push({
                    id: `${nodeId}:${snapshot.seq}:path`,
                    nodeId,
                    title,
                    label: "Legacy HLS",
                    url: legacyUrl,
                    type: "hls",
                    poster: payload.poster,
                });
            }
        }

        return entries;
    }

    let mode = $derived<PlayerMode>(item.config.mode === "message" ? "message" : "manual");
    let nodeId = $derived<string>(item.config.nodeId || "*");
    let manualSrc = $derived<string>(item.config.src || "");
    let manualType = $derived<SourceType>(item.config.sourceType || "hls");
    let autoplay = $derived<boolean>(!!item.config.autoplay);
    let muted = $derived<boolean>(!!item.config.muted);
    let poster = $derived<string>(item.config.poster || "");

    let streamSnapshots = $derived(
        context.snapshots
            .filter((snapshot: any) => snapshot.tag === "stream")
            .sort((a: any, b: any) => (b.seq ?? 0) - (a.seq ?? 0)),
    );
    let availableNodes = $derived(
        streamSnapshots
            .map((snapshot: any) => snapshot.node_id)
            .filter((value: string, index: number, items: string[]) => value && items.indexOf(value) === index),
    );
    let availableSources = $derived(
        streamSnapshots
            .filter((snapshot: any) => nodeId === "*" || snapshot.node_id === nodeId)
            .flatMap((snapshot: any) => extractSources(snapshot)),
    );

    let activeMessageSource = $derived(
        availableSources.find((source) => source.id === item.config.selectedSourceId) ?? availableSources[0] ?? null,
    );
    let playerSrc = $derived(mode === "manual" ? manualSrc : (activeMessageSource?.url ?? ""));
    let playerType = $derived<SourceType>(mode === "manual" ? manualType : (activeMessageSource?.type ?? "hls"));
    let playerPoster = $derived(mode === "manual" ? poster : (activeMessageSource?.poster ?? ""));
    let playerTitle = $derived(mode === "manual" ? "Manual Source" : (activeMessageSource?.title ?? "Message Source"));
    let playerSubtitle = $derived(mode === "manual" ? manualSrc : (activeMessageSource?.nodeId ?? ""));
    let playerKey = $derived(
        [playerSrc, playerType, String(autoplay), String(muted), playerPoster].join("|"),
    );

    function setMode(next: PlayerMode) {
        item.config.mode = next;
        updateConfig();
    }

    function setNodeFilter(value: string) {
        item.config.nodeId = value;
        if (!availableSources.some((source) => source.id === item.config.selectedSourceId)) {
            item.config.selectedSourceId = "";
        }
        updateConfig();
    }

    function setSelectedSource(value: string) {
        item.config.selectedSourceId = value;
        updateConfig();
    }
</script>

<div class="flex h-full w-full flex-col overflow-hidden bg-background">
    <div class="border-b bg-muted/15 px-3 py-2">
        <div class="flex items-center gap-2">
            <div class="inline-flex rounded-full bg-muted/30 p-0.5">
                <Button
                    variant={mode === "manual" ? "secondary" : "ghost"}
                    size="sm"
                    class="h-7 rounded-full px-2.5 text-[11px] font-mono"
                    onclick={() => setMode("manual")}
                >
                    Manual
                </Button>
                <Button
                    variant={mode === "message" ? "secondary" : "ghost"}
                    size="sm"
                    class="h-7 rounded-full px-2.5 text-[11px] font-mono"
                    onclick={() => setMode("message")}
                >
                    Message
                </Button>
            </div>

            {#if mode === "message"}
                <Select.Root type="single" value={nodeId} onValueChange={setNodeFilter}>
                    <Select.Trigger class="h-8 min-w-[130px] rounded-full border-border/50 bg-background px-3 text-xs shadow-none">
                        {nodeId === "*" ? "All Nodes" : nodeId}
                    </Select.Trigger>
                    <Select.Content>
                        <Select.Item value="*">All Nodes</Select.Item>
                        {#each availableNodes as availableNode}
                            <Select.Item value={availableNode}>{availableNode}</Select.Item>
                        {/each}
                    </Select.Content>
                </Select.Root>

                <Select.Root
                    type="single"
                    value={activeMessageSource?.id ?? ""}
                    onValueChange={setSelectedSource}
                >
                    <Select.Trigger class="h-8 min-w-[220px] flex-1 rounded-full border-border/50 bg-background px-3 text-xs shadow-none">
                        {activeMessageSource ? `${activeMessageSource.title} · ${activeMessageSource.label}` : "Select Source"}
                    </Select.Trigger>
                    <Select.Content>
                        {#each availableSources as source}
                            <Select.Item value={source.id}>
                                {source.title} · {source.label}
                            </Select.Item>
                        {/each}
                    </Select.Content>
                </Select.Root>
            {:else}
                <Input
                    value={manualSrc}
                    placeholder="https://example.com/live/index.m3u8"
                    class="h-8 flex-1 rounded-full border-border/50 bg-background text-xs shadow-none"
                    oninput={(event) => {
                        item.config.src = (event.currentTarget as HTMLInputElement).value;
                        updateConfig();
                    }}
                />
                <Select.Root
                    type="single"
                    value={manualType}
                    onValueChange={(value) => {
                        item.config.sourceType = value;
                        updateConfig();
                    }}
                >
                    <Select.Trigger class="h-8 min-w-[110px] rounded-full border-border/50 bg-background px-3 text-xs shadow-none">
                        {manualType.toUpperCase()}
                    </Select.Trigger>
                    <Select.Content>
                        <Select.Item value="hls">HLS</Select.Item>
                        <Select.Item value="video">Video</Select.Item>
                        <Select.Item value="audio">Audio</Select.Item>
                        <Select.Item value="auto">Auto</Select.Item>
                    </Select.Content>
                </Select.Root>
            {/if}
        </div>

        <div class="mt-2 flex items-center gap-4 text-[11px] text-muted-foreground">
            <label class="flex items-center gap-2">
                <Switch
                    checked={autoplay}
                    onCheckedChange={(value) => {
                        item.config.autoplay = value;
                        updateConfig();
                    }}
                />
                autoplay
            </label>
            <label class="flex items-center gap-2">
                <Switch
                    checked={muted}
                    onCheckedChange={(value) => {
                        item.config.muted = value;
                        updateConfig();
                    }}
                />
                muted
            </label>
        </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4">
        <section class="overflow-hidden rounded-xl border bg-background shadow-sm">
            <header class="border-b bg-muted/10 px-4 py-3">
                <div class="truncate text-[13px] font-medium">{playerTitle}</div>
                {#if playerSubtitle}
                    <div class="truncate font-mono text-[10px] text-muted-foreground">{playerSubtitle}</div>
                {/if}
            </header>

            <div class="space-y-3 p-4">
                <div class="aspect-video overflow-hidden rounded-lg border bg-black">
                    {#if playerSrc}
                        {#key playerKey}
                            <PlyrPlayer
                                src={playerSrc}
                                type={playerType}
                                poster={playerPoster}
                                {autoplay}
                                {muted}
                            />
                        {/key}
                    {:else}
                        <div class="flex h-full items-center justify-center text-sm text-muted-foreground">
                            {mode === "manual" ? "Enter a media URL." : "No playable sources found in stream messages."}
                        </div>
                    {/if}
                </div>

                <div class="space-y-1 text-[11px] text-muted-foreground">
                    <div class="font-mono uppercase tracking-[0.16em] text-[10px]">
                        {playerType}
                    </div>
                    {#if playerSrc}
                        <div class="truncate font-mono">{playerSrc}</div>
                    {/if}
                </div>
            </div>
        </section>
    </div>
</div>
