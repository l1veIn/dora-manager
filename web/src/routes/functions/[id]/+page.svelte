<script lang="ts">
    import { onMount } from "svelte";
    import { get, post } from "$lib/api";
    import { page } from "$app/state";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Box, ArrowLeft, Play, Check, AlertCircle } from "lucide-svelte";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Label } from "$lib/components/ui/label/index.js";

    let fn: any = $state(null);
    let loading = $state(true);
    let error = $state("");
    let selectedMethod = $state("");
    let inputJson = $state('{\n  \n}');
    let output: any = $state(null);
    let invoking = $state(false);
    let invokeError = $state("");

    let params = $derived.by(() => page.url.searchParams);
    let invokeMethodParam = $derived(params.get("invoke"));

    async function fetchFunction() {
        loading = true;
        try {
            fn = await get(`/fn/${page.params.id}`);
            if (fn.methods && fn.methods.length > 0) {
                selectedMethod = fn.methods[0].name;
            }
        } catch (e: any) {
            error = e.message || "Failed to load function";
            fn = null;
        } finally {
            loading = false;
        }
    }

    async function handleInvoke() {
        if (!selectedMethod) return;
        invoking = true;
        invokeError = "";
        output = null;
        try {
            let input: any = {};
            try {
                input = JSON.parse(inputJson);
            } catch {
                // use raw string as input
                input = inputJson;
            }
            let result = await post(`/fn/${fn.id}/invoke`, {
                method: selectedMethod,
                input,
            });
            output = result;
        } catch (e: any) {
            invokeError = e.message || "Invocation failed";
        } finally {
            invoking = false;
        }
    }

    onMount(() => {
        fetchFunction();
    });

    $effect(() => {
        if (!loading && fn && invokeMethodParam) {
            if (fn.methods?.find((m: any) => m.name === invokeMethodParam)) {
                selectedMethod = invokeMethodParam;
            }
        }
    });
</script>

<div class="p-6 max-w-4xl mx-auto space-y-6">
    <a
        href="/functions"
        class="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors"
    >
        <ArrowLeft class="size-4" />
        Back to Functions
    </a>

    {#if loading}
        <div class="animate-pulse space-y-4">
            <div class="h-8 w-48 bg-muted/50 rounded"></div>
            <div class="h-4 w-96 bg-muted/50 rounded"></div>
            <div class="h-48 bg-muted/50 rounded-lg"></div>
        </div>
    {:else if error}
        <div
            class="flex flex-col items-center justify-center p-12 text-center border rounded-lg bg-muted/10 h-64"
        >
            <AlertCircle class="h-12 w-12 text-destructive mb-4 opacity-50" />
            <h3 class="text-lg font-medium">Function not found</h3>
            <p class="text-sm text-muted-foreground mt-1">{error}</p>
        </div>
    {:else if fn}
        <div class="flex items-start justify-between">
            <div>
                <div class="flex items-center gap-2">
                    <Box class="size-5 text-primary" />
                    <h1 class="text-2xl font-bold tracking-tight">
                        {fn.name || fn.id}
                    </h1>
                    <Badge variant="secondary">v{fn.version}</Badge>
                </div>
                {#if fn.description}
                    <p class="text-muted-foreground mt-1">{fn.description}</p>
                {/if}
            </div>
            <Badge variant="outline" class="text-xs font-mono">{fn.entry}</Badge>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div class="space-y-4">
                <div class="rounded-lg border p-4 space-y-3">
                    <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                        Runtime
                    </h3>
                    <div class="grid grid-cols-2 gap-2 text-sm">
                        <span class="text-muted-foreground">Max Workers</span>
                        <span class="font-mono">{fn.runtime?.max_workers ?? 3}</span>
                        <span class="text-muted-foreground">Idle Timeout</span>
                        <span class="font-mono">{fn.runtime?.idle_timeout_secs ?? 300}s</span>
                    </div>
                </div>

                <div class="rounded-lg border p-4 space-y-3">
                    <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                        Methods
                    </h3>
                    <div class="space-y-2">
                        {#each fn.methods as method}
                            <div
                                class="flex items-center gap-2 p-2 rounded-md {selectedMethod === method.name
                                    ? 'bg-primary/10 border border-primary/30'
                                    : 'hover:bg-muted/50'} cursor-pointer"
                                onclick={() => (selectedMethod = method.name)}
                            >
                                <div class="flex-1 min-w-0">
                                    <div class="font-medium text-sm">
                                        {method.name}
                                    </div>
                                    {#if method.description}
                                        <div class="text-xs text-muted-foreground truncate">
                                            {method.description}
                                        </div>
                                    {/if}
                                </div>
                                {#if method.events && method.events.length > 0}
                                    <Badge variant="outline" class="text-xs">
                                        {method.events.length} events
                                    </Badge>
                                {/if}
                            </div>
                        {/each}
                    </div>
                </div>
            </div>

            <div class="space-y-4">
                <div class="rounded-lg border p-4 space-y-3">
                    <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                        Invoke
                    </h3>
                    <div class="space-y-3">
                        <div class="space-y-1.5">
                            <Label for="method">Method</Label>
                            <select
                                id="method"
                                class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm"
                                bind:value={selectedMethod}
                            >
                                {#each fn.methods as method}
                                    <option value={method.name}>
                                        {method.name}
                                    </option>
                                {/each}
                            </select>
                        </div>
                        <div class="space-y-1.5">
                            <Label for="input">Input (JSON)</Label>
                            <textarea
                                id="input"
                                class="flex min-h-[120px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm font-mono shadow-sm"
                                bind:value={inputJson}
                            ></textarea>
                        </div>
                        <Button
                            onclick={handleInvoke}
                            disabled={invoking || !selectedMethod}
                            class="w-full"
                        >
                            {#if invoking}
                                Invoking...
                            {:else}
                                <Play class="size-4 mr-2" />
                                Invoke {selectedMethod}
                            {/if}
                        </Button>
                    </div>
                </div>

                {#if output}
                    <div class="rounded-lg border p-4 space-y-2 bg-green-50 dark:bg-green-950/20 border-green-200 dark:border-green-800">
                        <div class="flex items-center gap-2 text-sm font-medium text-green-700 dark:text-green-400">
                            <Check class="size-4" />
                            Result
                        </div>
                        <pre class="text-xs font-mono bg-background rounded p-3 overflow-x-auto max-h-48 overflow-y-auto"><code>{JSON.stringify(output, null, 2)}</code></pre>
                    </div>
                {/if}

                {#if invokeError}
                    <div class="rounded-lg border p-4 space-y-2 bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-800">
                        <div class="flex items-center gap-2 text-sm font-medium text-red-700 dark:text-red-400">
                            <AlertCircle class="size-4" />
                            Error
                        </div>
                        <p class="text-sm text-red-600 dark:text-red-300">{invokeError}</p>
                    </div>
                {/if}
            </div>
        </div>
    {/if}
</div>
