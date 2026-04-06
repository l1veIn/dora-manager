<script lang="ts">
    let { file, runId } = $props<{ file: string; runId: string }>();
</script>

<audio controls class="max-w-full">
    <source src={`/api/runs/${runId}/artifacts/${file}`} />
</audio>
