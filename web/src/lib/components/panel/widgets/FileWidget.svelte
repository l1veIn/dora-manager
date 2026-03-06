<script lang="ts">
    import { FileDown } from "lucide-svelte";
    import { Button } from "$lib/components/ui/button/index.js";

    let { asset, runId } = $props<{
        asset: any;
        runId: string;
    }>();

    let fileUrl = $derived(`/api/runs/${runId}/panel/file/${asset.path}`);
</script>

<div
    class="flex items-center gap-3 p-3 rounded border bg-background/50 shadow-sm max-w-[300px]"
>
    <div
        class="h-10 w-10 shrink-0 bg-primary/10 text-primary rounded-md flex items-center justify-center"
    >
        <FileDown class="h-5 w-5" />
    </div>
    <div class="flex-1 min-w-0 pr-2">
        <div
            class="text-sm font-medium truncate"
            title={asset.path || "Attachment"}
        >
            {asset.path || "Binary Attachment"}
        </div>
        <div
            class="text-[10px] tabular-nums text-muted-foreground uppercase mt-0.5 max-w-[150px] truncate"
            title={asset.type}
        >
            {asset.type || "unknown"}
        </div>
    </div>
    <a href={fileUrl} download target="_blank" class="shrink-0">
        <Button variant="outline" size="sm" class="h-7 px-3 text-xs">
            Download
        </Button>
    </a>
</div>
