<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { get, post } from "$lib/api";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
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
    } from "lucide-svelte";

    // Config State
    let config = $state<any>(null);
    let versions = $state<any>(null);
    let doctor = $state<any>(null);

    // Operations
    let installVersionInput = $state("");
    let isInstalling = $state(false);
    let operations = $state<Record<string, "using" | "uninstalling">>({});

    async function loadData() {
        try {
            const [_config, _versions, _doctor] = await Promise.all([
                get("/config"),
                get("/versions"),
                get("/doctor"),
            ]);
            config = _config;
            versions = _versions;
            doctor = _doctor;
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
                                {#if doctor.python_installed}
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
                                >{doctor.python_path || "Not Found"}</span
                            >
                        </li>
                        <Separator />
                        <li class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                {#if doctor.uv_installed}
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
                                >{doctor.uv_path || "Not Found"}</span
                            >
                        </li>
                        <Separator />
                        <li class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                {#if doctor.dm_home_exists}
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
                        {#if versions?.installed_versions?.length > 0}
                            <div class="rounded-md border">
                                {#each versions.installed_versions as v, i}
                                    <div
                                        class="flex items-center justify-between p-4 {i !==
                                        versions.installed_versions.length - 1
                                            ? 'border-b'
                                            : ''}"
                                    >
                                        <div class="flex items-center gap-3">
                                            <span class="font-mono font-medium"
                                                >{v}</span
                                            >
                                            {#if v === versions.active_version}
                                                <Badge
                                                    variant="default"
                                                    class="bg-green-500 hover:bg-green-600"
                                                    >Active</Badge
                                                >
                                            {/if}
                                        </div>
                                        <div class="flex items-center gap-2">
                                            {#if v !== versions.active_version}
                                                <Button
                                                    variant="outline"
                                                    size="sm"
                                                    disabled={operations[v] ===
                                                        "using"}
                                                    onclick={() =>
                                                        useVersion(v)}
                                                >
                                                    {#if operations[v] === "using"}
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
                                                disabled={operations[v] ===
                                                    "uninstalling" ||
                                                    v ===
                                                        versions.active_version}
                                                onclick={() =>
                                                    uninstallVersion(v)}
                                                title={v ===
                                                versions.active_version
                                                    ? "Cannot uninstall active version"
                                                    : ""}
                                            >
                                                {#if operations[v] === "uninstalling"}
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
