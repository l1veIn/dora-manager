<script lang="ts">
    let { entry } = $props<{ entry: any }>();

    const payload = $derived(entry.payload ?? {});
    const target = $derived(
        payload.to
            ? `${payload.to}${payload.output_id ? `.${payload.output_id}` : ""}`
            : null,
    );
    const value = $derived(payload.value);
    const formattedValue = $derived.by(() => {
        if (typeof value === "string") return value;
        if (
            typeof value === "number" ||
            typeof value === "boolean" ||
            value === null ||
            value === undefined
        ) {
            return String(value ?? "");
        }
        return JSON.stringify(value, null, 2);
    });
</script>

<div class="flex w-full justify-end">
    <div class="flex max-w-[85%] flex-col items-end gap-1">
        <div class="flex items-center gap-1.5 px-1">
            <span class="text-[9px] text-muted-foreground/50">
                {new Date((entry.timestamp || 0) * 1000).toLocaleTimeString()}
            </span>
            <span class="text-[10px] font-mono font-medium tracking-tight text-muted-foreground/70">
                web
            </span>
        </div>
        <div class="w-fit rounded-2xl rounded-tr-sm border border-primary/15 bg-primary/8 px-3 py-2 text-sm text-foreground shadow-sm backdrop-blur-[1px]">
            <div class="font-mono text-[11px] whitespace-pre-wrap break-words leading-relaxed">
                {formattedValue}
            </div>
            {#if target}
                <div class="mt-2 text-right text-[9px] uppercase tracking-[0.16em] text-muted-foreground/70">
                    to {target}
                </div>
            {/if}
        </div>
    </div>
</div>
