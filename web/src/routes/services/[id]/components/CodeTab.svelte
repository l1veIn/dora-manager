<script lang="ts">
    import { Search, RefreshCw, FileText, Terminal } from "lucide-svelte";
    import { Input } from "$lib/components/ui/input/index.js";
    import FileTree from "./FileTree.svelte";
    import CodeMirror from "svelte-codemirror-editor";
    import { mode } from "mode-watcher";
    import { oneDark } from "@codemirror/theme-one-dark";

    // Extensions
    import { python } from "@codemirror/lang-python";
    import { javascript } from "@codemirror/lang-javascript";
    import { json } from "@codemirror/lang-json";
    import { markdown } from "@codemirror/lang-markdown";
    import { yaml } from "@codemirror/lang-yaml";
    import { rust } from "@codemirror/lang-rust";

    let {
        files = [],
        loadingFiles = false,
        selectedFile = null,
        selectedFileContent = "",
        loadingFileContent = false,
        onSelectFile = () => {},
        parsedMarkdown = () => "",
    } = $props<{
        files: string[];
        loadingFiles?: boolean;
        selectedFile?: string | null;
        selectedFileContent?: string;
        loadingFileContent?: boolean;
        onSelectFile?: (file: string) => void;
        parsedMarkdown?: (content: string) => string;
    }>();

    let fileSearch = $state("");

    $effect(() => {
        if (!selectedFile && files.length > 0) {
            const readme = files.find(
                (f: string) => f.toLowerCase() === "readme.md",
            );
            if (readme && onSelectFile) {
                onSelectFile(readme);
            }
        }
    });

    // Build the hierarchical tree from the flat array of files
    function buildFileTree(paths: string[]) {
        const root: any[] = [];

        paths.forEach((path) => {
            const parts = path.split("/");
            let currentLevel = root;

            parts.forEach((part, index) => {
                const isFile = index === parts.length - 1;
                const existingPath = parts.slice(0, index + 1).join("/");

                let existingService = currentLevel.find(
                    (n: any) => n.name === part,
                );

                if (existingService) {
                    if (!isFile) {
                        currentLevel = (existingService as any).children;
                    }
                } else {
                    const newService = isFile
                        ? { name: part, path: existingPath, type: "file" }
                        : {
                              name: part,
                              path: existingPath,
                              type: "directory",
                              children: [] as any[],
                          };

                    currentLevel.push(newService);
                    if (!isFile) {
                        currentLevel = (newService as any).children;
                    }
                }
            });
        });

        // Sort: directories first, then alphabetical
        const sortServices = (services: any[]) => {
            services.sort((a, b) => {
                if (a.type === b.type) return a.name.localeCompare(b.name);
                return a.type === "directory" ? -1 : 1;
            });
            services.forEach((n: any) => {
                if (n.type === "directory" && n.children) sortServices(n.children);
            });
        };
        sortServices(root);
        return root;
    }

    let filteredTree = $derived(() => {
        let filteredPaths = files;
        if (fileSearch.trim()) {
            const lowerSearch = fileSearch.toLowerCase();
            filteredPaths = files.filter((f: string) =>
                f.toLowerCase().includes(lowerSearch),
            );
        }
        return buildFileTree(filteredPaths);
    });

    let currentExtension = $derived(() => {
        if (!selectedFile) return [];
        const ext = selectedFile.split(".").pop()?.toLowerCase();
        switch (ext) {
            case "py":
                return [python()];
            case "js":
            case "ts":
                return [javascript()];
            case "json":
                return [json()];
            case "yml":
            case "yaml":
                return [yaml()];
            case "rs":
                return [rust()];
            case "md":
                return [markdown()];
            default:
                return [];
        }
    });

    let isMarkdown = $derived(selectedFile?.toLowerCase().endsWith(".md"));
</script>

<!-- CODE TAB -->
<div
    class="flex-1 grid grid-cols-1 md:grid-cols-[300px_1fr] gap-6 h-full min-h-0 overflow-hidden"
>
    <!-- File Tree Sidebar -->
    <div class="border rounded-md bg-card flex flex-col max-h-full min-h-0">
        <div class="p-3 border-b bg-muted/20 flex items-center gap-2">
            <Search class="size-4 text-muted-foreground" />
            <Input
                placeholder="Find file..."
                bind:value={fileSearch}
                class="h-8 shadow-none border-none bg-transparent px-2 placeholder:text-muted-foreground/70 focus-visible:ring-0"
            />
        </div>
        <div class="overflow-y-auto flex-1 p-2">
            {#if loadingFiles}
                <div class="flex justify-center p-8 opacity-50">
                    <RefreshCw class="size-5 animate-spin" />
                </div>
            {:else if files.length === 0}
                <div class="text-center p-4 text-sm text-muted-foreground">
                    No files found
                </div>
            {:else}
                <FileTree
                    services={filteredTree()}
                    {selectedFile}
                    onSelect={onSelectFile}
                />
            {/if}
        </div>
    </div>

    <!-- File Content Area -->
    <div
        class="border rounded-md bg-card flex flex-col overflow-hidden max-h-full min-h-0"
    >
        {#if selectedFile}
            <div
                class="p-3 border-b bg-muted/20 flex items-center justify-between font-mono text-sm shrink-0"
            >
                <div class="flex items-center gap-2">
                    <FileText class="size-4 text-muted-foreground" />
                    <span class="font-medium">{selectedFile}</span>
                </div>
            </div>

            <div class="flex-1 bg-stone-50 dark:bg-stone-950 px-0 relative">
                {#if loadingFileContent}
                    <div
                        class="absolute inset-0 flex items-center justify-center bg-background/50 backdrop-blur-sm z-10 w-full h-full"
                    >
                        <RefreshCw
                            class="size-6 animate-spin text-muted-foreground"
                        />
                    </div>
                {/if}

                <div
                    class="absolute inset-0 flex flex-col overflow-auto bg-stone-50 dark:bg-stone-950"
                >
                    {#if isMarkdown}
                        <div
                            class="prose prose-sm md:prose-base dark:prose-invert max-w-none prose-pre:bg-stone-950 prose-pre:text-stone-300 prose-pre:border p-6"
                        >
                            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                            {@html parsedMarkdown(selectedFileContent)}
                        </div>
                    {:else}
                        <CodeMirror
                            value={selectedFileContent}
                            readonly={true}
                            lineWrapping={true}
                            theme={mode && mode.current === "dark"
                                ? oneDark
                                : undefined}
                            extensions={currentExtension()}
                            styles={{
                                "&": {
                                    flex: "1",
                                    height: "100%",
                                    minHeight: "100%",
                                    backgroundColor: "transparent",
                                },
                            }}
                        />
                    {/if}
                </div>
            </div>
        {:else}
            <div
                class="flex-1 flex flex-col items-center justify-center text-muted-foreground bg-muted/5"
            >
                <Terminal class="size-12 mb-4 opacity-20" />
                <p>Select a file to view its contents</p>
            </div>
        {/if}
    </div>
</div>
