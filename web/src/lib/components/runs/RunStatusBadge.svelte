<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";

    let { status, stopRequestedAt = null } = $props<{
        status: string;
        stopRequestedAt?: string | null;
    }>();
</script>

{#if status === "running" && stopRequestedAt}
    <Badge
        variant="outline"
        class="font-mono border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-300"
        >stopping</Badge
    >
{:else if status === "running"}
    <Badge
        variant="outline"
        class="font-mono bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/40 dark:text-blue-400 dark:border-blue-800"
        >running</Badge
    >
{:else if status === "succeeded"}
    <Badge
        variant="default"
        class="font-mono bg-emerald-600 hover:bg-emerald-700 text-white border-transparent"
        >succeeded</Badge
    >
{:else if status === "stopped"}
    <Badge
        variant="secondary"
        class="font-mono bg-muted/50 text-muted-foreground border-transparent"
        >stopped</Badge
    >
{:else if status === "failed"}
    <Badge
        variant="destructive"
        class="font-mono bg-red-600 hover:bg-red-700 text-white border-transparent"
        >failed</Badge
    >
{:else}
    <Badge variant="outline" class="font-mono">{status}</Badge>
{/if}
