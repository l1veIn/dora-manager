<script lang="ts">
    import { post } from "$lib/api";
    import { toast } from "svelte-sonner";
    import CodeMirror from "svelte-codemirror-editor";
    import { yaml } from "@codemirror/lang-yaml";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Save } from "lucide-svelte";
    import { mode } from "mode-watcher";
    import { oneDark } from "@codemirror/theme-one-dark";

    let {
        dataflowName,
        initialYaml = "",
        onCodeUpdated,
    } = $props<{
        dataflowName: string;
        initialYaml?: string;
        onCodeUpdated?: (newYaml: string) => void;
    }>();

    // svelte-ignore state_referenced_locally
    let code = $state(initialYaml);
    let isSaving = $state(false);
    // svelte-ignore state_referenced_locally
    let lastHydratedYaml = $state(initialYaml);

    $effect(() => {
        // Hydrate only when initialYaml changes from parent substantially (e.g. after history restore)
        if (initialYaml && initialYaml !== lastHydratedYaml) {
            code = initialYaml;
            lastHydratedYaml = initialYaml;
        }
    });

    async function saveDataflow() {
        if (!dataflowName || !code) return;
        isSaving = true;
        try {
            await post(`/dataflows/${dataflowName}`, { yaml: code });
            toast.success("YAML Saved successfully");
            onCodeUpdated?.(code);
        } catch (e: any) {
            toast.error(`Save failed: ${e.message}`);
        } finally {
            isSaving = false;
        }
    }
</script>

<div class="h-full flex flex-col min-h-0 w-full">
    <!-- Action Bar for Yaml -->
    <div class="flex justify-end mb-3">
        <Button
            variant="outline"
            size="sm"
            disabled={isSaving || code === initialYaml}
            onclick={saveDataflow}
        >
            <Save class="mr-2 size-4" />
            {isSaving ? "Saving..." : "Save code"}
        </Button>
    </div>

    <!-- Editor Container -->
    <div
        class="flex-1 min-h-0 relative border rounded-md shadow-sm overflow-scroll group"
    >
        <div
            class="absolute inset-0 [&_.cm-editor]:h-full [&_.cm-scroller]:font-mono [&_.cm-scroller]:text-[13.5px]"
        >
            <CodeMirror
                bind:value={code}
                lang={yaml()}
                theme={mode && mode.current === "dark" ? oneDark : undefined}
                styles={{
                    "&": {
                        height: "100%",
                        width: "100%",
                        backgroundColor: "transparent",
                        color: "inherit",
                    },
                    ".cm-gutters": {
                        backgroundColor: "transparent",
                        borderRight: "1px solid hsl(var(--border))",
                    },
                    "&.cm-focused": {
                        outline: "none",
                    },
                }}
            />
        </div>
    </div>
</div>
