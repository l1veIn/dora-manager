<script lang="ts">
    import {
        ChevronRight,
        ChevronDown,
        File,
        FileText,
        Folder,
        FolderOpen,
        BookOpen,
    } from "lucide-svelte";
    import FileTree from "./FileTree.svelte";

    let {
        services = [],
        selectedFile = null,
        onSelect = () => {},
    } = $props<{
        services: any[];
        selectedFile: string | null;
        onSelect: (path: string) => void;
    }>();

    // Map to keep track of which folders are expanded
    let expandedMap = $state<Record<string, boolean>>({});

    function toggleService(service: any, e: Event) {
        e.stopPropagation();
        if (service.type === "directory") {
            expandedMap[service.path] = !expandedMap[service.path];
        } else {
            onSelect(service.path);
        }
    }
</script>

<div class="flex flex-col gap-0.5 font-mono text-sm">
    {#each services as service}
        <div class="flex flex-col">
            <button
                class="flex items-center gap-1.5 px-2 py-1.5 rounded-sm hover:bg-muted text-left w-full transition-colors group {selectedFile ===
                service.path
                    ? 'bg-accent text-accent-foreground font-medium'
                    : 'text-muted-foreground'}"
                onclick={(e) => toggleService(service, e)}
            >
                {#if service.type === "directory"}
                    <div
                        class="size-4 shrink-0 flex items-center justify-center text-muted-foreground/60 group-hover:text-muted-foreground transition-colors"
                    >
                        {#if expandedMap[service.path]}
                            <ChevronDown class="size-3.5" />
                        {:else}
                            <ChevronRight class="size-3.5" />
                        {/if}
                    </div>
                {:else}
                    <div class="size-4 shrink-0"></div>
                {/if}

                <div class="shrink-0">
                    {#if service.type === "directory"}
                        {#if expandedMap[service.path]}
                            <FolderOpen
                                class="size-3.5 opacity-70 {selectedFile ===
                                service.path
                                    ? 'text-accent-foreground'
                                    : 'text-blue-500/80 dark:text-blue-400/80'}"
                            />
                        {:else}
                            <Folder
                                class="size-3.5 opacity-70 {selectedFile ===
                                service.path
                                    ? 'text-accent-foreground'
                                    : 'text-blue-500/80 dark:text-blue-400/80'}"
                            />
                        {/if}
                    {:else if service.name.endsWith(".md")}
                        <BookOpen class="size-3.5 opacity-70" />
                    {:else if service.name.includes(".")}
                        <FileText class="size-3.5 opacity-70" />
                    {:else}
                        <File class="size-3.5 opacity-70" />
                    {/if}
                </div>

                <span class="truncate">{service.name}</span>
            </button>

            {#if service.type === "directory" && expandedMap[service.path]}
                <div class="ml-3 pl-2 border-l border-muted-foreground/20">
                    <FileTree services={service.children} {selectedFile} {onSelect} />
                </div>
            {/if}
        </div>
    {/each}
</div>
