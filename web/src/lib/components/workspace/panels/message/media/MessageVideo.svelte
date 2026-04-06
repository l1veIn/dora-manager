<script lang="ts">
    let { file, runId } = $props<{ file: string; runId: string }>();
</script>

<video controls class="max-h-[280px] max-w-full rounded border bg-black">
    <source src={`/api/runs/${runId}/artifacts/${file}`} />
</video>
