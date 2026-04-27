<script lang="ts">
    import { page } from "$app/state";
    import { get, post, getText } from "$lib/api";
    import { goto } from "$app/navigation";
    import { toast } from "svelte-sonner";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Skeleton } from "$lib/components/ui/skeleton/index.js";
    import * as Tooltip from "$lib/components/ui/tooltip/index.js";
    import {
        RefreshCw,
        ArrowLeft,
        Trash2,
        Play,
        Terminal,
        Box,
        Settings,
        FolderOpen,
        ChevronDown,
        UserRound,
    } from "lucide-svelte";
    import * as DropdownUI from "$lib/components/ui/dropdown-menu/index.js";
    import {
        isInstalledService,
        serviceAvatarSrc,
        serviceCategory,
        serviceOriginLabel,
        servicePrimaryMaintainer,
        serviceRuntimeLabel,
    } from "$lib/services/catalog";

    // Markdown
    import { marked } from "marked";
    import DOMPurify from "dompurify";

    import CodeTab from "./components/CodeTab.svelte";
    import InvokeTab from "./components/InvokeTab.svelte";
    import SettingsTab from "./components/SettingsTab.svelte";

    // Build the highlight component logic here to avoid huge dependencies if possible
    // We can just use standard pre formatting for now until required

    let serviceId = $derived(page.params.id);
    let isNewService = $derived(page.url.searchParams.get("new") === "1");
    let service = $state<any>(null);
    let loading = $state(true);

    // File Browser state
    let files = $state<string[]>([]);
    let loadingFiles = $state(false);
    let selectedFile = $state<string | null>(null);
    let selectedFileContent = $state<string>("");
    let loadingFileContent = $state(false);

    // Config state
    let configSchema = $state<any>(null);
    let originalConfig = $state<any>({});
    let formData = $state<Record<string, any>>({});
    let loadingConfig = $state(false);
    let savingConfig = $state(false);

    // Invocation state
    let selectedMethod = $state("");
    let inputJson = $state("{}");
    let outputJson = $state("");
    let invoking = $state(false);

    // Actions state
    let operation = $state<string | null>(null);

    $effect(() => {
        if (serviceId) {
            loadServiceDetails();
        }
    });

    async function loadServiceDetails() {
        loading = true;
        try {
            const status: any = await get(`/services/${serviceId}`);
            if (!status) {
                toast.error(`Service ${serviceId} not found`);
                goto("/services");
                return;
            }
            service = status;
            configSchema = status?.config_schema || null;

            // Load all other details in parallel
            await Promise.all([loadFiles(), loadConfig()]);
        } catch (e) {
            console.error("Failed to load service:", e);
            toast.error("Failed to load service details");
        } finally {
            loading = false;
        }
    }

    function parseMarkdown(md: string) {
        if (!md || md === "Loading...") {
            return md;
        }
        try {
            return DOMPurify.sanitize(marked.parse(md) as string);
        } catch (e) {
            return `<pre class="whitespace-pre-wrap">${md}</pre>`;
        }
    }

    async function loadFiles() {
        loadingFiles = true;
        try {
            const res: any = await get(`/services/${serviceId}/files`);
            if (Array.isArray(res)) {
                files = res.sort();
            }
        } catch (e) {
            console.error("Failed to load files:", e);
        } finally {
            loadingFiles = false;
        }
    }

    async function handleFileSelect(file: string) {
        selectedFile = file;
        loadingFileContent = true;
        selectedFileContent = "";

        try {
            // Because paths can contain slashes, we fetch as text.
            const content = await getText(`/services/${serviceId}/files/${file}`);
            selectedFileContent = content;
        } catch (e: any) {
            selectedFileContent = `Error loading file:\n${e.message || "Unknown error"}`;
        } finally {
            loadingFileContent = false;
        }
    }

    async function loadConfig() {
        loadingConfig = true;
        try {
            const cfg = await get(`/services/${serviceId}/config`);
            originalConfig = cfg || {};
            formData = { ...originalConfig };

            if (configSchema) {
                Object.keys(configSchema).forEach((key) => {
                    if (
                        formData[key] === undefined &&
                        configSchema[key].default !== undefined
                    ) {
                        formData[key] = configSchema[key].default;
                    }
                });
            }
        } catch (e) {
            console.error("Failed to load config", e);
        } finally {
            loadingConfig = false;
        }
    }

    async function saveConfig() {
        savingConfig = true;
        try {
            await post(`/services/${serviceId}/config`, formData);
            toast.success("Configuration saved");
            originalConfig = { ...formData };
        } catch (e: any) {
            toast.error(`Failed to save config: ${e.message}`);
        } finally {
            savingConfig = false;
        }
    }

    async function handleInstall() {
        operation = "installing";
        try {
            await post("/services/install", { id: serviceId });
            toast.success(`${serviceId} installed successfully`);
            await loadServiceDetails();
        } catch (e: any) {
            toast.error(`Failed to install: ${e.message}`);
        } finally {
            operation = null;
        }
    }

    async function handleUninstall() {
        if (!confirm(`Are you sure you want to delete ${serviceId}?`)) return;
        operation = "uninstalling";
        try {
            await post("/services/uninstall", { id: serviceId });
            toast.success(`${serviceId} uninstalled`);
            goto("/services");
        } catch (e: any) {
            toast.error(`Failed to uninstall: ${e.message}`);
            operation = null;
        }
    }

    let needsInstall = $derived(!isInstalledService(service));
    let avatarBroken = $state(false);
    let avatarSrc = $derived(
        (() => {
            avatarBroken;
            return avatarBroken ? null : serviceAvatarSrc(service);
        })(),
    );

    async function openExternally(target: "finder" | "terminal" | "vscode") {
        try {
            await post(`/services/${serviceId}/open`, { target });
            toast.success(`Opened ${serviceId} in ${target}`);
        } catch (e: any) {
            toast.error(`Failed to open ${serviceId}: ${e.message}`);
        }
    }

    let serviceMethods = $derived(Array.isArray(service?.methods) ? service.methods : []);
    let hasMethodsOverview = $derived(serviceMethods.length > 0);
    let activeTab = $state<"code" | "invoke" | "config">("code");

    $effect(() => {
        if (!selectedMethod && serviceMethods.length > 0) {
            selectedMethod = serviceMethods[0].name;
            inputJson = JSON.stringify(defaultInputForMethod(serviceMethods[0]), null, 2);
        }
    });

    function defaultInputForMethod(method: any) {
        const properties = method?.input_schema?.properties;
        if (!properties || typeof properties !== "object") {
            return {};
        }

        return Object.fromEntries(
            Object.entries(properties).map(([key, schema]: [string, any]) => {
                if (schema?.default !== undefined) return [key, schema.default];
                if (schema?.type === "number" || schema?.type === "integer") return [key, 1];
                if (schema?.type === "boolean") return [key, false];
                if (schema?.type === "array") return [key, []];
                if (schema?.type === "object") return [key, {}];
                return [key, ""];
            }),
        );
    }

    async function invokeSelectedMethod() {
        if (!selectedMethod) return;

        invoking = true;
        try {
            const input = JSON.parse(inputJson || "{}");
            const result = await post(`/services/${serviceId}/invoke`, {
                method: selectedMethod,
                input,
            });
            outputJson = JSON.stringify(result, null, 2);
            toast.success(`${serviceId}.${selectedMethod} completed`);
        } catch (e: any) {
            outputJson = e?.message || String(e);
            toast.error(`Failed to invoke service: ${e?.message || e}`);
        } finally {
            invoking = false;
        }
    }
</script>

<div
    class="flex flex-col h-full pt-6 pb-6 px-4 md:px-8 w-full max-w-7xl mx-auto space-y-6 min-h-0"
>
    <!-- Breadcrumb and Actions Header -->
    <!-- Title Bar: Breadcrumb and Actions -->
    <div class="flex items-center justify-between">
        <Button
            variant="ghost"
            size="sm"
            class="w-fit -ml-2 text-muted-foreground hover:text-foreground"
            href="/services"
        >
            <ArrowLeft class="size-4 mr-1" />
            Back to Services
        </Button>

        {#if service}
            <div class="flex items-center gap-2">
                <DropdownUI.Root>
                    <DropdownUI.Trigger
                        class="inline-flex items-center justify-center rounded-md border border-input bg-background px-4 py-2 text-sm font-medium shadow-sm transition-colors hover:bg-accent hover:text-accent-foreground gap-2"
                    >
                        <FolderOpen class="size-4" />
                        Open With
                        <ChevronDown class="size-4" />
                    </DropdownUI.Trigger>
                    <DropdownUI.Content align="end">
                        <DropdownUI.Item
                            onclick={() => openExternally("vscode")}
                        >
                            Open in VS Code
                        </DropdownUI.Item>
                        <DropdownUI.Item
                            onclick={() => openExternally("finder")}
                        >
                            Open in Finder
                        </DropdownUI.Item>
                        <DropdownUI.Item
                            onclick={() => openExternally("terminal")}
                        >
                            Open in Terminal
                        </DropdownUI.Item>
                    </DropdownUI.Content>
                </DropdownUI.Root>
                {#if !service.builtin}
                    {#if needsInstall}
                        <Button
                            disabled={operation === "installing"}
                            onclick={handleInstall}
                        >
                            {#if operation === "installing"}
                                <RefreshCw class="size-4 animate-spin mr-2" />
                            {:else}
                                <Play class="size-4 mr-2" />
                            {/if}
                            Install Service
                        </Button>
                    {:else}
                        <Button
                            variant="outline"
                            disabled={operation === "installing"}
                            onclick={handleInstall}
                            title="Re-install this service (e.g. after code changes)"
                        >
                            {#if operation === "installing"}
                                <RefreshCw class="size-4 animate-spin mr-2" />
                            {:else}
                                <RefreshCw class="size-4 mr-2" />
                            {/if}
                            Re-install
                        </Button>
                    {/if}
                    <Button
                        variant="destructive"
                        class="gap-2"
                        disabled={operation === "uninstalling"}
                        onclick={handleUninstall}
                    >
                        {#if operation === "uninstalling"}
                            <RefreshCw class="size-4 animate-spin" />
                        {:else}
                            <Trash2 class="size-4" />
                        {/if}
                        Delete
                    </Button>
                {/if}
            </div>
        {/if}
    </div>

    <!-- Service Info Header -->
    <div class="flex flex-col gap-3">
        <div class="flex items-center gap-4 min-w-0">
            <div
                class="h-14 w-14 rounded-xl border bg-muted/40 overflow-hidden flex items-center justify-center shrink-0"
            >
                {#if avatarSrc}
                    <img
                        src={avatarSrc}
                        alt={`${serviceId} avatar`}
                        class="h-full w-full object-cover"
                        onerror={() => (avatarBroken = true)}
                    />
                {:else}
                    <Box class="size-6 text-primary" />
                {/if}
            </div>
            <div class="flex flex-col gap-1.5 min-w-0">
                <div class="flex items-center gap-3 flex-wrap">
                    <h1
                        class="text-3xl font-bold font-mono tracking-tight truncate"
                    >
                        {serviceId}
                    </h1>
                    {#if service}
                        <Badge
                            variant="outline"
                            class="font-mono text-xs rounded-full px-3 py-0.5 shadow-sm"
                        >
                            {service.version || "v0.0.0"}
                        </Badge>
                    {/if}
                </div>
                {#if service}
                    <div class="flex items-center gap-2 flex-wrap mt-0.5">
                        <div
                            class="text-[11px] font-medium text-muted-foreground flex items-center gap-1.5 pr-2"
                        >
                            <UserRound class="size-3" />
                            {servicePrimaryMaintainer(service) ||
                                "Unknown maintainer"}
                        </div>
                        <Badge
                            variant="secondary"
                            class="text-xs rounded-full shadow-sm"
                        >
                            {serviceRuntimeLabel(service)}
                        </Badge>
                        {#if serviceOriginLabel(service) && serviceOriginLabel(service) !== "unknown"}
                            <Badge
                                variant="outline"
                                class="text-xs rounded-full shadow-sm"
                            >
                                {serviceOriginLabel(service)}
                            </Badge>
                        {/if}
                        {#if serviceCategory(service) && serviceCategory(service) !== "uncategorized"}
                            <Badge
                                variant="outline"
                                class="text-xs rounded-full shadow-sm"
                            >
                                {serviceCategory(service)}
                            </Badge>
                        {/if}
                        {#if service.scope}
                            <Badge
                                variant="outline"
                                class="text-xs rounded-full shadow-sm"
                            >
                                {service.scope}
                            </Badge>
                        {/if}

                        <div
                            class="h-3 w-[1px] bg-border/60 mx-1 hidden sm:block"
                        ></div>
                    </div>
                {/if}
            </div>
        </div>
        {#if service?.description}
            <p class="text-muted-foreground mt-1 text-sm leading-relaxed">
                {service.description}
            </p>
        {/if}
    </div>

    {#if loading}
        <div class="space-y-4">
            <Skeleton class="h-10 w-full max-w-md" />
            <Skeleton class="h-[60vh] w-full rounded-lg" />
        </div>
    {:else}
        {#if isNewService}
            <div
                class="rounded-lg border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-950"
            >
                <p class="font-medium">Service scaffold created</p>
                <p class="mt-1">
                    Continue in the code tab, or use
                    <span class="font-medium">Open With</span> to jump straight into
                    VS Code, Finder, or Terminal.
                </p>
                {#if service?.path}
                    <p class="mt-2 text-xs">
                        Scaffold location:
                        <span class="font-mono">{service.path}</span>
                    </p>
                {/if}
            </div>
        {/if}
        <div class="min-w-0 space-y-6">
            {#if hasMethodsOverview}
                <Tooltip.Provider>
                    <section>
                        <div
                            class="flex flex-col gap-3 lg:flex-row lg:items-start"
                        >
                            <div class="flex min-w-0 flex-1 flex-col gap-2">
                                <div class="flex flex-wrap gap-2">
                                    {#each serviceMethods as method}
                                        <Tooltip.Root>
                                            <Tooltip.Trigger>
                                                {#snippet child({ props })}
                                                    <button
                                                        type="button"
                                                        class="inline-flex max-w-full items-center gap-2 rounded-md border bg-muted/15 px-2.5 py-1 text-left text-[11px] text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
                                                        {...props}
                                                    >
                                                        <span class="font-mono text-foreground">
                                                            {method.name}
                                                        </span>
                                                    </button>
                                                {/snippet}
                                            </Tooltip.Trigger>
                                            <Tooltip.Content
                                                side="top"
                                                align="start"
                                                class="max-w-xs whitespace-pre-line"
                                            >
                                                {method.description || "No description provided."}
                                            </Tooltip.Content>
                                        </Tooltip.Root>
                                    {/each}
                                </div>
                            </div>
                        </div>
                    </section>
                </Tooltip.Provider>
            {/if}

            <!-- Main Content Area -->
            <Tabs.Root
                bind:value={activeTab}
                class="flex flex-col min-h-0 min-w-0 rounded-xl border bg-card p-4 md:p-5 h-[clamp(34rem,68vh,54rem)]"
            >
                <Tabs.List class="w-full mb-4 flex flex-wrap gap-2">
                    <Tabs.Trigger value="code" class="gap-2">
                        <Terminal class="size-4" />
                        Code
                    </Tabs.Trigger>
                    <Tabs.Trigger value="invoke" class="gap-2">
                        <Play class="size-4" />
                        Invoke
                    </Tabs.Trigger>
                    <Tabs.Trigger value="config" class="gap-2">
                        <Settings class="size-4" />
                        Settings
                    </Tabs.Trigger>
                </Tabs.List>

                <!-- CODE TAB -->
                <Tabs.Content
                    value="code"
                    class="flex-1 flex flex-col min-h-0 overflow-hidden"
                >
                    <CodeTab
                        {files}
                        {loadingFiles}
                        {selectedFile}
                        {selectedFileContent}
                        {loadingFileContent}
                        onSelectFile={handleFileSelect}
                        parsedMarkdown={parseMarkdown}
                    />
                </Tabs.Content>

                <Tabs.Content
                    value="invoke"
                    class="flex-1 flex flex-col min-h-0 overflow-hidden"
                >
                    <InvokeTab
                        methods={serviceMethods}
                        bind:selectedMethod
                        bind:inputJson
                        {outputJson}
                        {invoking}
                        onInvoke={invokeSelectedMethod}
                    />
                </Tabs.Content>

                <!-- CONFIG TAB -->
                <Tabs.Content
                    value="config"
                    class="flex-1 border rounded-md bg-card flex flex-col min-h-0 overflow-hidden"
                >
                    <SettingsTab
                        {configSchema}
                        {originalConfig}
                        bind:formData
                        {loadingConfig}
                        {savingConfig}
                        onSave={saveConfig}
                        onRevert={() => (formData = { ...originalConfig })}
                    />
                </Tabs.Content>
            </Tabs.Root>
        </div>
    {/if}
</div>
