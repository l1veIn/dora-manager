<script lang="ts">
    let { asset } = $props<{
        asset: any;
        runId?: string;
    }>();
</script>

<div
    class="whitespace-pre-wrap break-all font-sans text-[13px] leading-relaxed select-text"
>
    {asset.data || ""}
</div>
