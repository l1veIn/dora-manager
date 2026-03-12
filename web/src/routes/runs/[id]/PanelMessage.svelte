<script lang="ts">
    import TextWidget from "./panel/display/TextWidget.svelte";
    import JsonWidget from "./panel/display/JsonWidget.svelte";
    import ImageWidget from "./panel/display/ImageWidget.svelte";
    import AudioWidget from "./panel/display/AudioWidget.svelte";
    import VideoWidget from "./panel/display/VideoWidget.svelte";
    import FileWidget from "./panel/display/FileWidget.svelte";

    let { asset, runId } = $props<{
        asset: any;
        runId: string;
    }>();

    let isUser = $derived(asset.source === "user");
    let isCommand = $derived(
        asset.input_id?.startsWith("command/") ||
            asset.type === "command/text",
    );
    let isSent = $derived(isUser || isCommand);
    let commandLabel = $derived(
        isCommand ? asset.input_id?.replace("command/", "") : null,
    );

    // Map content types to widget components
    let Widget = $derived(
        !asset.type
            ? FileWidget
            : asset.type === "command/text" || asset.type.startsWith("text/")
              ? TextWidget
              : asset.type === "application/json"
                ? JsonWidget
                : asset.type.startsWith("image/")
                  ? ImageWidget
                  : asset.type.startsWith("audio/")
                    ? AudioWidget
                    : asset.type.startsWith("video/")
                      ? VideoWidget
                      : FileWidget,
    );

    // Formatting timestamp
    let formattedTime = $derived(() => {
        if (!asset.timestamp) return "";
        return new Date(asset.timestamp).toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
        });
    });
</script>

<div class="flex flex-col gap-1 {isSent ? 'items-end' : 'items-start w-full'}">
    <!-- Sender Attribution -->
    {#if isCommand && commandLabel}
        <span
            class="text-[10px] text-blue-500/70 dark:text-blue-400/70 font-medium px-1 uppercase tracking-wider"
        >
            ▶ {commandLabel}
        </span>
    {:else if asset.input_id && !isSent}
        <span
            class="text-[10px] text-muted-foreground/60 font-medium px-1 uppercase tracking-wider"
        >
            {asset.input_id}
        </span>
    {/if}

    <!-- Chat Bubble Container -->
    <div
        class="p-3 rounded-lg border text-sm max-w-[85%] shadow-sm {isSent
            ? 'bg-blue-50/50 dark:bg-blue-950/20 border-blue-100 dark:border-blue-900'
            : 'bg-muted/20'}"
    >
        <!-- Bubble Header (Type & Time) -->
        <div
            class="font-mono text-[10px] text-muted-foreground mb-1.5 flex items-center justify-between gap-4"
        >
            <span class="uppercase tracking-wider font-semibold"
                >{isCommand ? "sent" : asset.type}</span
            >
            <span class="opacity-50 min-w-16 text-right">{formattedTime()}</span
            >
        </div>

        <!-- Dynamic Widget Content -->
        <Widget {asset} {runId} />
    </div>
</div>
