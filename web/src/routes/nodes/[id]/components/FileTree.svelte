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
        nodes = [],
        selectedFile = null,
        onSelect = () => {},
    } = $props<{
        nodes: any[];
        selectedFile: string | null;
        onSelect: (path: string) => void;
    }>();

    // Map to keep track of which folders are expanded
    let expandedMap = $state<Record<string, boolean>>({});

    function toggleNode(node: any, e: Event) {
        e.stopPropagation();
        if (node.type === "directory") {
            expandedMap[node.path] = !expandedMap[node.path];
        } else {
            onSelect(node.path);
        }
    }
</script>

<div class="flex flex-col gap-0.5 font-mono text-sm">
    {#each nodes as node}
        <div class="flex flex-col">
            <button
                class="flex items-center gap-1.5 px-2 py-1.5 rounded-sm hover:bg-muted text-left w-full transition-colors group {selectedFile ===
                node.path
                    ? 'bg-accent text-accent-foreground font-medium'
                    : 'text-muted-foreground'}"
                onclick={(e) => toggleNode(node, e)}
            >
                {#if node.type === "directory"}
                    <div
                        class="size-4 shrink-0 flex items-center justify-center text-muted-foreground/60 group-hover:text-muted-foreground transition-colors"
                    >
                        {#if expandedMap[node.path]}
                            <ChevronDown class="size-3.5" />
                        {:else}
                            <ChevronRight class="size-3.5" />
                        {/if}
                    </div>
                {:else}
                    <div class="size-4 shrink-0"></div>
                {/if}

                <div class="shrink-0">
                    {#if node.type === "directory"}
                        {#if expandedMap[node.path]}
                            <FolderOpen
                                class="size-3.5 opacity-70 {selectedFile ===
                                node.path
                                    ? 'text-accent-foreground'
                                    : 'text-blue-500/80 dark:text-blue-400/80'}"
                            />
                        {:else}
                            <Folder
                                class="size-3.5 opacity-70 {selectedFile ===
                                node.path
                                    ? 'text-accent-foreground'
                                    : 'text-blue-500/80 dark:text-blue-400/80'}"
                            />
                        {/if}
                    {:else if node.name.endsWith(".md")}
                        <BookOpen class="size-3.5 opacity-70" />
                    {:else if node.name.includes(".")}
                        <FileText class="size-3.5 opacity-70" />
                    {:else}
                        <File class="size-3.5 opacity-70" />
                    {/if}
                </div>

                <span class="truncate">{node.name}</span>
            </button>

            {#if node.type === "directory" && expandedMap[node.path]}
                <div class="ml-3 pl-2 border-l border-muted-foreground/20">
                    <FileTree nodes={node.children} {selectedFile} {onSelect} />
                </div>
            {/if}
        </div>
    {/each}
</div>
