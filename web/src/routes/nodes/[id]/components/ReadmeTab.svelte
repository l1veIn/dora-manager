<script lang="ts">
    import { Skeleton } from "$lib/components/ui/skeleton/index.js";

    let { loadingReadme = false, parsedReadme = () => "" } = $props<{
        loadingReadme?: boolean;
        parsedReadme: () => string;
    }>();
</script>

{#if loadingReadme}
    <div class="flex flex-col space-y-3">
        <Skeleton class="h-4 w-3/4" />
        <Skeleton class="h-4 w-1/2" />
        <Skeleton class="h-[200px] w-full" />
    </div>
{:else}
    <div
        class="prose prose-sm md:prose-base dark:prose-invert max-w-none prose-pre:bg-stone-950 prose-pre:text-stone-300 prose-pre:border"
    >
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html parsedReadme()}
    </div>
{/if}
