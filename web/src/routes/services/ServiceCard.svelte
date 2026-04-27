<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Separator } from "$lib/components/ui/separator/index.js";
    import { RefreshCw, Download, Trash2, UserRound } from "lucide-svelte";
    import {
        isInstalledService,
        serviceAvatarSrc,
        serviceCategory,
        serviceOriginLabel,
        servicePrimaryMaintainer,
        serviceRuntimeLabel,
    } from "$lib/services/catalog";

    let {
        service,
        operation = null,
        onAction,
        href,
    } = $props<{
        service: any;
        operation?: string | null;
        onAction?: (action: string, id: string) => void;
        href?: string;
    }>();

    import { MoreVertical } from "lucide-svelte";
    import * as DropdownUI from "$lib/components/ui/dropdown-menu/index.js";

    let needsInstall = $derived(!isInstalledService(service));
    let avatarBroken = $state(false);
    let avatarSrc = $derived(avatarBroken ? null : serviceAvatarSrc(service));
</script>

<svelte:element
    this={href ? "a" : "div"}
    {href}
    role={href ? undefined : "button"}
    tabindex={href ? undefined : 0}
    class="group relative w-full text-left border rounded-2xl p-5 flex flex-col bg-card/40 backdrop-blur-sm hover:bg-card hover:shadow-xl hover:shadow-primary/5 hover:border-primary/20 transition-all duration-300 {operation
        ? 'opacity-50 pointer-events-none'
        : ''} focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 {href
        ? ''
        : 'cursor-pointer'} block hover:no-underline text-foreground overflow-hidden"
>
    <!-- Subtle gradient glow effect on hover -->
    <div class="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"></div>

    <!-- Header -->
    <div class="relative z-10 flex items-start justify-between w-full gap-4">
        <div class="flex items-start gap-4 min-w-0 flex-1">
            <div
                class="h-14 w-14 rounded-xl border bg-gradient-to-br from-muted/50 to-muted/20 shadow-sm overflow-hidden flex items-center justify-center shrink-0 group-hover:ring-2 ring-primary/10 transition-all duration-300"
            >
                {#if avatarSrc}
                    <img
                        src={avatarSrc}
                        alt={`${service.name || service.id} avatar`}
                        class="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
                        onerror={() => (avatarBroken = true)}
                    />
                {:else}
                    <UserRound class="size-6 text-primary/60 group-hover:text-primary transition-colors" />
                {/if}
            </div>
            <div class="pr-2 min-w-0 space-y-1.5 flex-1 mt-0.5">
                <div class="flex items-center justify-between">
                    <div class="font-bold font-mono text-lg tracking-tight truncate flex items-center gap-2 group-hover:text-primary transition-colors">
                        <span class="truncate">{service.name || service.id}</span>
                        {#if href}
                            <svg class="size-3.5 opacity-0 -translate-x-2 translate-y-2 group-hover:opacity-100 group-hover:translate-x-0 group-hover:translate-y-0 transition-all duration-300" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 7h10v10"/><path d="M7 17 17 7"/></svg>
                        {/if}
                    </div>
                </div>
                <div class="flex items-center gap-1.5 flex-wrap">
                    <Badge
                        variant="outline"
                        class="font-mono text-[10px] rounded-full px-2 py-0 h-5 whitespace-nowrap bg-background/50 backdrop-blur-md"
                    >
                        {serviceOriginLabel(service)}
                    </Badge>
                    <Badge variant="secondary" class="text-[10px] rounded-full px-2 py-0 h-5 bg-secondary/50">
                        {serviceCategory(service)}
                    </Badge>
                    {#if needsInstall}
                        <Badge variant="outline" class="text-[10px] rounded-full px-2 py-0 h-5 border-destructive/30 text-destructive bg-destructive/10">
                            Not Installed
                        </Badge>
                    {:else}
                        <Badge
                            variant="outline"
                            class="text-[10px] rounded-full px-2 py-0 h-5 bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:border-emerald-500/20"
                        >
                            Installed
                        </Badge>
                    {/if}
                </div>
            </div>
        </div>

        <div class="flex flex-col items-end gap-2 relative z-20">
            {#if onAction && !service.builtin}
                <DropdownUI.Root>
                    <DropdownUI.Trigger
                        class="h-8 w-8 -mr-2 text-muted-foreground hover:text-foreground inline-flex items-center justify-center rounded-full text-sm font-medium whitespace-nowrap transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:shadow-sm"
                    >
                        <span class="sr-only">Open menu</span>
                        <MoreVertical class="h-4 w-4" />
                    </DropdownUI.Trigger>
                    <DropdownUI.Content align="end" class="rounded-xl shadow-lg border-border/50 bg-background/95 backdrop-blur-md">
                        {#if needsInstall}
                            <DropdownUI.Item
                                class="rounded-lg cursor-pointer"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    e.preventDefault();
                                    onAction("install", service.id);
                                }}
                            >
                                <Download class="mr-2 h-4 w-4 text-primary" />
                                <span>Install</span>
                            </DropdownUI.Item>
                        {:else}
                            <DropdownUI.Item
                                class="rounded-lg cursor-pointer"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    e.preventDefault();
                                    onAction("install", service.id);
                                }}
                            >
                                <RefreshCw class="mr-2 h-4 w-4 text-primary" />
                                <span>Re-install</span>
                            </DropdownUI.Item>
                        {/if}
                        <DropdownUI.Separator />
                        <DropdownUI.Item
                            class="text-destructive focus:text-destructive rounded-lg cursor-pointer"
                            onclick={(e) => {
                                e.stopPropagation();
                                e.preventDefault();
                                onAction("uninstall", service.id);
                            }}
                        >
                            <Trash2 class="mr-2 h-4 w-4" />
                            <span>Delete</span>
                        </DropdownUI.Item>
                    </DropdownUI.Content>
                </DropdownUI.Root>
            {/if}
            <Badge
                variant="outline"
                class="font-mono text-[10px] rounded-full bg-background/50 text-muted-foreground border-border/50 whitespace-nowrap flex-shrink-0"
            >
                {service.version || "v0.0.0"}
            </Badge>
        </div>
    </div>

    <!-- Description -->
    <div class="relative z-10 text-sm text-muted-foreground/90 mt-4 line-clamp-2 min-h-[2.5rem] w-full leading-relaxed">
        {service.description || "No description provided."}
    </div>

    <Separator class="my-4 opacity-50 relative z-10" />

    <!-- Footer Status -->
    <div class="relative z-10 mt-auto space-y-3 w-full">
        <div class="flex items-center gap-1.5 flex-wrap">
            <Badge variant="secondary" class="text-[10px] rounded-full px-2 py-0 h-5 bg-secondary/50">
                {serviceRuntimeLabel(service)}
            </Badge>
            {#if service.display?.tags?.length}
                {#each service.display.tags.slice(0, 2) as tag}
                    <Badge variant="outline" class="text-[10px] rounded-full px-2 py-0 h-5 bg-background/50 border-border/50 text-muted-foreground">{tag}</Badge>
                {/each}
            {/if}
        </div>

        <div class="flex items-center justify-between gap-2">
            <div class="text-xs text-muted-foreground/80 flex items-center gap-1.5 min-w-0 group-hover:text-muted-foreground transition-colors">
                <div class="p-1 rounded-full bg-muted/50">
                    <UserRound class="size-3 shrink-0" />
                </div>
                <span class="truncate font-medium">{servicePrimaryMaintainer(service) || "Unknown maintainer"}</span>
            </div>

            <div class="flex items-center gap-2">
                {#if operation === "downloading"}
                    <span class="text-xs text-primary font-medium flex items-center bg-primary/10 px-2 py-1 rounded-full"
                        ><RefreshCw class="size-3 animate-spin mr-1.5" /> Downloading...</span
                    >
                {:else if operation === "installing"}
                    <span class="text-xs text-primary font-medium flex items-center bg-primary/10 px-2 py-1 rounded-full"
                        ><RefreshCw class="size-3 animate-spin mr-1.5" /> Installing...</span
                    >
                {:else if operation === "uninstalling"}
                    <span class="text-xs text-destructive font-medium flex items-center bg-destructive/10 px-2 py-1 rounded-full"
                        ><RefreshCw class="size-3 animate-spin mr-1.5" /> Uninstalling...</span
                    >
                {/if}
            </div>
        </div>
    </div>
</svelte:element>
