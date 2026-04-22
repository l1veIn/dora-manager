<script lang="ts">
    let { entry } = $props<{ entry: any }>();

    const payload = $derived(entry.payload ?? {});
    const widgets = $derived(Object.entries(payload.widgets ?? {}) as Array<[string, any]>);
    const label = $derived(payload.label ?? entry.from);
    const summary = $derived.by(() => {
        if (widgets.length === 0) {
            return `${label} registered controls`;
        }
        if (widgets.length === 1) {
            const [outputId, widget] = widgets[0];
            return `${label} registered ${widget?.type ?? "input"} control for ${outputId}`;
        }
        return `${label} registered ${widgets.length} controls`;
    });
    const detail = $derived.by(() =>
        widgets
            .map(([outputId, widget]) => `${widget?.label ?? outputId} · ${widget?.type ?? "control"}`)
            .join("\n"),
    );
</script>

<div class="flex w-full items-center justify-center py-1">
    <div
        class="max-w-[80%] text-center text-[10px] uppercase tracking-[0.18em] text-muted-foreground/55"
        title={detail}
    >
        -- {summary} --
    </div>
</div>
