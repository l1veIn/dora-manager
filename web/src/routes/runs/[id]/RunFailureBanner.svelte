<script lang="ts">
    import { AlertTriangle } from "lucide-svelte";

    let { failureNode, failureMessage } = $props<{
        failureNode?: string;
        failureMessage?: string;
    }>();
</script>

{#if failureNode || failureMessage}
    <div
        class="border border-red-200 bg-red-50 dark:bg-red-950/20 dark:border-red-900 text-red-800 dark:text-red-400 p-4 rounded-md flex items-start gap-3 shadow-sm mb-4"
    >
        <AlertTriangle
            class="size-5 shrink-0 mt-0.5 text-red-600 dark:text-red-500"
        />
        <div class="flex flex-col gap-1 w-full">
            <h3 class="font-semibold text-sm text-red-900 dark:text-red-300">
                Run Failed
            </h3>
            <p class="text-sm break-words whitespace-pre-wrap">
                {failureMessage || "Unknown error occurred."}
            </p>
            {#if failureNode}
                <div class="mt-2 flex items-center gap-2 text-xs">
                    <span class="font-medium text-red-900 dark:text-red-300"
                        >Failure Node:</span
                    >
                    <span
                        class="font-mono bg-red-100 dark:bg-red-900/50 px-1.5 py-0.5 rounded text-red-900 dark:text-red-200"
                        >{failureNode}</span
                    >
                </div>
            {/if}
        </div>
    </div>
{/if}
