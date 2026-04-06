<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { get, post } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import * as Card from "$lib/components/ui/card/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { toast } from "svelte-sonner";
    import {
        Settings2,
        Download,
        Trash2,
        Power,
        CheckCircle2,
        XCircle,
        RefreshCw,
        Server,
        AlertTriangle,
    } from "lucide-svelte";

    // Config State
    let config = $state<any>(null);
    let versions = $state<any>(null);
    let doctor = $state<any>(null);
    let mediaStatus = $state<any>(null);
    let savingMedia = $state(false);
    let installingMedia = $state(false);
    let mediaForm = $state<any>({
        enabled: false,
        backend: "media_mtx",
        mediamtx: {
            path: "",
            version: "",
            auto_download: true,
            host: "127.0.0.1",
            public_host: "",
            api_port: 9997,
            rtsp_port: 8554,
            hls_port: 8888,
            webrtc_port: 8889,
            public_webrtc_url: "",
            public_hls_url: "",
        },
    });

    // Operations
    let installVersionInput = $state("");
    let isInstalling = $state(false);
    let operations = $state<Record<string, "using" | "uninstalling">>({});

    async function loadData() {
        try {
            const [_config, _versions, _doctor, _mediaStatus] =
                (await Promise.all([
                    get("/config"),
                    get("/versions"),
                    get("/doctor"),
                    get("/media/status"),
                ])) as [any, any, any, any];
            config = _config;
            versions = _versions;
            doctor = _doctor;
            mediaStatus = _mediaStatus;
            mediaForm = {
                enabled: !!_config?.media?.enabled,
                backend: _config?.media?.backend || "media_mtx",
                mediamtx: {
                    path: _config?.media?.mediamtx?.path || "",
                    version: _config?.media?.mediamtx?.version || "",
                    auto_download:
                        _config?.media?.mediamtx?.auto_download ?? true,
                    host: _config?.media?.mediamtx?.host || "127.0.0.1",
                    public_host: _config?.media?.mediamtx?.public_host || "",
                    api_port: _config?.media?.mediamtx?.api_port ?? 9997,
                    rtsp_port: _config?.media?.mediamtx?.rtsp_port ?? 8554,
                    hls_port: _config?.media?.mediamtx?.hls_port ?? 8888,
                    webrtc_port: _config?.media?.mediamtx?.webrtc_port ?? 8889,
                    public_webrtc_url:
                        _config?.media?.mediamtx?.public_webrtc_url || "",
                    public_hls_url:
                        _config?.media?.mediamtx?.public_hls_url || "",
                },
            };
        } catch (e: any) {
            toast.error(`Failed to load settings: ${e.message}`);
        }
    }

    onMount(() => {
        loadData();
    });

    async function installVersion() {
        if (!installVersionInput.trim()) return;

        isInstalling = true;
        toast.info(`Installing Dora version ${installVersionInput}...`);
        try {
            await post("/install", { version: installVersionInput.trim() });
            toast.success(`Installed version ${installVersionInput}`);
            installVersionInput = "";
            await loadData();
        } catch (e: any) {
            toast.error(`Failed to install version: ${e.message}`);
        } finally {
            isInstalling = false;
        }
    }

    async function useVersion(v: string) {
        operations[v] = "using";
        try {
            await post("/use", { version: v });
            toast.success(`Switched to version ${v}`);
            await loadData();
        } catch (e: any) {
            toast.error(`Failed to switch version: ${e.message}`);
        } finally {
            delete operations[v];
        }
    }

    async function uninstallVersion(v: string) {
        if (!confirm(`Are you sure you want to uninstall version ${v}?`))
            return;

        operations[v] = "uninstalling";
        try {
            await post("/uninstall", { version: v });
            toast.success(`Uninstalled version ${v}`);
            await loadData();
        } catch (e: any) {
            toast.error(`Failed to uninstall version: ${e.message}`);
        } finally {
            delete operations[v];
        }
    }

    function normalizeMediaPayload() {
        return {
            enabled: !!mediaForm.enabled,
            backend: "media_mtx",
            mediamtx: {
                path: mediaForm.mediamtx.path?.trim() || null,
                version: mediaForm.mediamtx.version?.trim() || null,
                auto_download: !!mediaForm.mediamtx.auto_download,
                host: mediaForm.mediamtx.host?.trim() || "127.0.0.1",
                public_host: mediaForm.mediamtx.public_host?.trim() || null,
                api_port: Number(mediaForm.mediamtx.api_port) || 9997,
                rtsp_port: Number(mediaForm.mediamtx.rtsp_port) || 8554,
                hls_port: Number(mediaForm.mediamtx.hls_port) || 8888,
                webrtc_port: Number(mediaForm.mediamtx.webrtc_port) || 8889,
                public_webrtc_url:
                    mediaForm.mediamtx.public_webrtc_url?.trim() || null,
                public_hls_url:
                    mediaForm.mediamtx.public_hls_url?.trim() || null,
            },
        };
    }

    async function saveMediaSettings() {
        savingMedia = true;
        try {
            config = await post("/config", {
                active_version: config?.active_version ?? null,
                media: normalizeMediaPayload(),
            });
            toast.success(
                "Saved media settings. Restart dm-server to apply port or backend changes.",
            );
            await loadData();
        } catch (e: any) {
            toast.error(`Failed to save media settings: ${e.message}`);
        } finally {
            savingMedia = false;
        }
    }

    async function installMedia() {
        installingMedia = true;
        try {
            mediaStatus = await post("/media/install");
            toast.success("MediaMTX resolved successfully.");
        } catch (e: any) {
            toast.error(`Failed to install media backend: ${e.message}`);
        } finally {
            installingMedia = false;
        }
    }
</script>

<div class="p-6 max-w-4xl mx-auto space-y-6">
    <div>
        <h1 class="text-3xl font-bold tracking-tight">Settings</h1>
        <p class="text-sm text-muted-foreground">
            Manage your Dora environment and configuration.
        </p>
    </div>

    <div class="grid gap-6">
        <!-- Environment / Doctor -->
        <Card.Root>
            <Card.Header>
                <Card.Title class="flex items-center gap-2"
                    ><Settings2 class="size-5" /> Environment Health</Card.Title
                >
                <Card.Description
                    >Checks your system dependencies required for Dora.</Card.Description
                >
            </Card.Header>
            <Card.Content>
                {#if doctor}
                    <ul class="space-y-3">
                        <li class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                {#if doctor.python?.found}
                                    <CheckCircle2
                                        class="text-green-500 size-4"
                                    />
                                {:else}
                                    <XCircle class="text-red-500 size-4" />
                                {/if}
                                <span class="font-medium text-sm">Python 3</span
                                >
                            </div>
                            <span
                                class="text-xs text-muted-foreground font-mono bg-muted/50 px-2 py-1 rounded"
                                >{doctor.python?.path || "Not Found"}</span
                            >
                        </li>
                        <Separator />
                        <li class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                {#if doctor.uv?.found}
                                    <CheckCircle2
                                        class="text-green-500 size-4"
                                    />
                                {:else}
                                    <XCircle class="text-red-500 size-4" />
                                {/if}
                                <span class="font-medium text-sm"
                                    >uv Package Manager</span
                                >
                            </div>
                            <span
                                class="text-xs text-muted-foreground font-mono bg-muted/50 px-2 py-1 rounded"
                                >{doctor.uv?.path || "Not Found"}</span
                            >
                        </li>
                        <Separator />
                        <li class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                {#if doctor}
                                    <CheckCircle2
                                        class="text-green-500 size-4"
                                    />
                                {:else}
                                    <XCircle class="text-red-500 size-4" />
                                {/if}
                                <span class="font-medium text-sm"
                                    >dm Home Directory</span
                                >
                            </div>
                            <span
                                class="text-xs text-muted-foreground font-mono bg-muted/50 px-2 py-1 rounded"
                                >~/.dm</span
                            >
                        </li>
                    </ul>
                {:else}
                    <div class="text-sm text-muted-foreground">
                        Loading environment data...
                    </div>
                {/if}
            </Card.Content>
        </Card.Root>

        <!-- Version Management -->
        <Card.Root>
            <Card.Header>
                <Card.Title>Dora Versions</Card.Title>
                <Card.Description
                    >Install and switch between different Dora runtime versions.</Card.Description
                >
            </Card.Header>
            <Card.Content>
                <div class="space-y-6">
                    <div class="flex gap-2">
                        <Input
                            type="text"
                            placeholder="e.g. 0.3.9, 0.4.1, main"
                            bind:value={installVersionInput}
                            class="max-w-xs"
                        />
                        <Button
                            onclick={installVersion}
                            disabled={isInstalling ||
                                !installVersionInput.trim()}
                        >
                            {#if isInstalling}
                                <RefreshCw class="size-4 animate-spin mr-2" /> Installing
                            {:else}
                                <Download class="size-4 mr-2" /> Install Version
                            {/if}
                        </Button>
                    </div>

                    <Separator />

                    <div class="space-y-4">
                        <h3 class="text-sm font-medium">Installed Versions</h3>
                        {#if versions?.installed?.length > 0}
                            <div class="rounded-md border">
                                {#each versions.installed as v, i}
                                    <div
                                        class="flex items-center justify-between p-4 {i !==
                                        versions.installed.length - 1
                                            ? 'border-b'
                                            : ''}"
                                    >
                                        <div class="flex items-center gap-3">
                                            <span class="font-mono font-medium"
                                                >{v.version}</span
                                            >
                                            {#if v.active}
                                                <Badge
                                                    variant="default"
                                                    class="bg-green-500 hover:bg-green-600"
                                                    >Active</Badge
                                                >
                                            {/if}
                                        </div>
                                        <div class="flex items-center gap-2">
                                            {#if !v.active}
                                                <Button
                                                    variant="outline"
                                                    size="sm"
                                                    disabled={operations[
                                                        v.version
                                                    ] === "using"}
                                                    onclick={() =>
                                                        useVersion(v.version)}
                                                >
                                                    {#if operations[v.version] === "using"}
                                                        <RefreshCw
                                                            class="size-4 animate-spin mr-2"
                                                        /> Switching
                                                    {:else}
                                                        <Power
                                                            class="size-4 mr-2"
                                                        /> Use
                                                    {/if}
                                                </Button>
                                            {/if}
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                class="text-red-500 hover:text-red-600 hover:bg-red-500/10"
                                                disabled={operations[
                                                    v.version
                                                ] === "uninstalling" ||
                                                    v.active}
                                                onclick={() =>
                                                    uninstallVersion(v.version)}
                                                title={v.active
                                                    ? "Cannot uninstall active version"
                                                    : ""}
                                            >
                                                {#if operations[v.version] === "uninstalling"}
                                                    <RefreshCw
                                                        class="size-4 animate-spin mr-2"
                                                    />
                                                {:else}
                                                    <Trash2 class="size-4" />
                                                {/if}
                                            </Button>
                                        </div>
                                    </div>
                                {/each}
                            </div>
                        {:else}
                            <div
                                class="text-sm text-muted-foreground p-4 bg-muted/30 rounded-md border border-dashed text-center"
                            >
                                No versions installed.
                            </div>
                        {/if}
                    </div>

                    <Separator />

                    <div class="space-y-4">
                        <h3 class="text-sm font-medium">Available Versions</h3>
                        {#if versions?.available?.length > 0}
                            <div class="rounded-md border">
                                {#each versions.available as v, i}
                                    <div
                                        class="flex items-center justify-between p-3 {i !==
                                        versions.available.length - 1
                                            ? 'border-b'
                                            : ''}"
                                    >
                                        <span class="font-mono text-sm"
                                            >{v.tag}</span
                                        >
                                        {#if v.installed}
                                            <Badge variant="secondary"
                                                >Installed</Badge
                                            >
                                        {:else}
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                disabled={isInstalling}
                                                onclick={() => {
                                                    installVersionInput = v.tag;
                                                    installVersion();
                                                }}
                                            >
                                                <Download class="size-4 mr-1" />
                                                Install
                                            </Button>
                                        {/if}
                                    </div>
                                {/each}
                            </div>
                        {:else}
                            <div
                                class="text-sm text-muted-foreground p-4 bg-muted/30 rounded-md border border-dashed text-center"
                            >
                                Unable to fetch available versions.
                            </div>
                        {/if}
                    </div>
                </div>
            </Card.Content>
        </Card.Root>

        <Card.Root>
            <Card.Header>
                <Card.Title class="flex items-center gap-2"
                    ><Server class="size-5" /> Media Backend</Card.Title
                >
                <Card.Description
                    >Manage MediaMTX for streaming panels and media-capable nodes.</Card.Description
                >
            </Card.Header>
            <Card.Content>
                <div class="space-y-6">
                    <div class="flex items-start justify-between gap-4 rounded-lg border p-4">
                        <div class="space-y-2">
                            <div class="flex items-center gap-2">
                                <span class="font-medium">Runtime Status</span>
                                {#if mediaStatus}
                                    <Badge
                                        variant="outline"
                                        class={mediaStatus.status === "ready"
                                            ? "border-green-200 bg-green-50 text-green-700"
                                            : mediaStatus.status === "disabled"
                                              ? "border-slate-200 bg-slate-50 text-slate-700"
                                              : "border-amber-200 bg-amber-50 text-amber-700"}
                                    >
                                        {mediaStatus.status}
                                    </Badge>
                                {/if}
                            </div>
                            <p class="text-sm text-muted-foreground">
                                {mediaStatus?.message ||
                                    "Media backend has not been initialized yet."}
                            </p>
                            {#if mediaStatus?.binary_path}
                                <p class="text-xs font-mono text-muted-foreground break-all">
                                    {mediaStatus.binary_path}
                                </p>
                            {/if}
                        </div>
                        <div class="flex items-center gap-2">
                            <Button
                                variant="outline"
                                onclick={installMedia}
                                disabled={installingMedia}
                            >
                                {#if installingMedia}
                                    <RefreshCw class="size-4 animate-spin mr-2" /> Resolving
                                {:else}
                                    <Download class="size-4 mr-2" /> Install / Resolve
                                {/if}
                            </Button>
                            <Button
                                onclick={saveMediaSettings}
                                disabled={savingMedia}
                            >
                                {#if savingMedia}
                                    <RefreshCw class="size-4 animate-spin mr-2" /> Saving
                                {:else}
                                    Save Media Settings
                                {/if}
                            </Button>
                        </div>
                    </div>

                    <div
                        class="rounded-lg border border-amber-200 bg-amber-50/60 p-3 text-sm text-amber-900"
                    >
                        <div class="flex items-start gap-2">
                            <AlertTriangle class="mt-0.5 size-4 shrink-0" />
                            <p>
                                Config changes are persisted immediately, but
                                port, path, and backend changes require a
                                `dm-server` restart before the runtime picks
                                them up.
                            </p>
                        </div>
                    </div>

                    <div class="grid gap-6 md:grid-cols-2">
                        <div class="space-y-3">
                            <div class="flex items-center justify-between">
                                <div class="space-y-0.5">
                                    <Label for="media-enabled"
                                        >Enable media backend</Label
                                    >
                                    <p class="text-xs text-muted-foreground">
                                        Required for `stream` and `video`
                                        panels backed by MediaMTX.
                                    </p>
                                </div>
                                <Switch
                                    checked={mediaForm.enabled}
                                    onCheckedChange={(checked) =>
                                        (mediaForm.enabled = !!checked)}
                                />
                            </div>

                            <div class="space-y-2">
                                <Label for="mediamtx-version"
                                    >Preferred MediaMTX version</Label
                                >
                                <Input
                                    id="mediamtx-version"
                                    placeholder="e.g. 1.11.3"
                                    bind:value={mediaForm.mediamtx.version}
                                />
                            </div>

                            <div class="space-y-2">
                                <Label for="mediamtx-path"
                                    >Binary path override</Label
                                >
                                <Input
                                    id="mediamtx-path"
                                    placeholder="/path/to/mediamtx"
                                    bind:value={mediaForm.mediamtx.path}
                                />
                                <p class="text-xs text-muted-foreground">
                                    Leave empty to use cached auto-downloads.
                                </p>
                            </div>

                            <div class="flex items-center justify-between">
                                <div class="space-y-0.5">
                                    <Label for="mediamtx-auto-download"
                                        >Auto-download missing binary</Label
                                    >
                                    <p class="text-xs text-muted-foreground">
                                        Fetch the matching GitHub release into
                                        `DM_HOME/bin/mediamtx`.
                                    </p>
                                </div>
                                <Switch
                                    checked={mediaForm.mediamtx.auto_download}
                                    onCheckedChange={(checked) =>
                                        (mediaForm.mediamtx.auto_download =
                                            !!checked)}
                                />
                            </div>
                        </div>

                        <div class="space-y-3">
                            <div class="grid grid-cols-2 gap-3">
                                <div class="space-y-2">
                                    <Label for="mediamtx-host">Bind host</Label>
                                    <Input
                                        id="mediamtx-host"
                                        bind:value={mediaForm.mediamtx.host}
                                    />
                                </div>
                                <div class="space-y-2">
                                    <Label for="mediamtx-public-host"
                                        >Public host</Label
                                    >
                                    <Input
                                        id="mediamtx-public-host"
                                        placeholder="optional"
                                        bind:value={mediaForm.mediamtx.public_host}
                                    />
                                </div>
                            </div>

                            <div class="grid grid-cols-2 gap-3">
                                <div class="space-y-2">
                                    <Label for="mediamtx-api-port"
                                        >API port</Label
                                    >
                                    <Input
                                        id="mediamtx-api-port"
                                        type="number"
                                        bind:value={mediaForm.mediamtx.api_port}
                                    />
                                </div>
                                <div class="space-y-2">
                                    <Label for="mediamtx-rtsp-port"
                                        >RTSP port</Label
                                    >
                                    <Input
                                        id="mediamtx-rtsp-port"
                                        type="number"
                                        bind:value={mediaForm.mediamtx.rtsp_port}
                                    />
                                </div>
                                <div class="space-y-2">
                                    <Label for="mediamtx-hls-port"
                                        >HLS port</Label
                                    >
                                    <Input
                                        id="mediamtx-hls-port"
                                        type="number"
                                        bind:value={mediaForm.mediamtx.hls_port}
                                    />
                                </div>
                                <div class="space-y-2">
                                    <Label for="mediamtx-webrtc-port"
                                        >WebRTC port</Label
                                    >
                                    <Input
                                        id="mediamtx-webrtc-port"
                                        type="number"
                                        bind:value={mediaForm.mediamtx.webrtc_port}
                                    />
                                </div>
                            </div>

                            <div class="space-y-2">
                                <Label for="mediamtx-public-webrtc"
                                    >Public WebRTC URL override</Label
                                >
                                <Input
                                    id="mediamtx-public-webrtc"
                                    placeholder="optional"
                                    bind:value={mediaForm.mediamtx.public_webrtc_url}
                                />
                            </div>

                            <div class="space-y-2">
                                <Label for="mediamtx-public-hls"
                                    >Public HLS URL override</Label
                                >
                                <Input
                                    id="mediamtx-public-hls"
                                    placeholder="optional"
                                    bind:value={mediaForm.mediamtx.public_hls_url}
                                />
                            </div>
                        </div>
                    </div>
                </div>
            </Card.Content>
        </Card.Root>

        <!-- Raw Config Viewer -->
        <Card.Root>
            <Card.Header>
                <Card.Title>Raw Configuration</Card.Title>
            </Card.Header>
            <Card.Content>
                {#if config}
                    <div
                        class="bg-slate-950 text-slate-50 p-4 rounded-md overflow-x-auto text-xs font-mono"
                    >
                        <pre>{JSON.stringify(config, null, 2)}</pre>
                    </div>
                {:else}
                    <div class="text-sm text-muted-foreground">
                        Loading configuration...
                    </div>
                {/if}
            </Card.Content>
        </Card.Root>
    </div>
</div>
