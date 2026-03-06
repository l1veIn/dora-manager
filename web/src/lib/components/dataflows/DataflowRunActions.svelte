<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { Play, Square, ChevronDown, Rocket } from "lucide-svelte";

    let {
        activeRunId = null,
        isStarting = false,
        onRun = (force: boolean) => {},
        onStop = () => {},
        onViewRun = () => {},
    } = $props<{
        activeRunId?: string | null;
        isStarting: boolean;
        onRun: (force: boolean) => void;
        onStop: () => void;
        onViewRun?: () => void;
    }>();
</script>

<div class="flex items-center gap-2">
    {#if activeRunId}
        <Button
            variant="outline"
            size="sm"
            onclick={onViewRun}
            class="bg-blue-50/50 text-blue-700 hover:bg-blue-100 hover:text-blue-800 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 font-mono text-xs"
        >
            Viewing Run {activeRunId.substring(0, 8)}...
        </Button>
        <Button variant="destructive" size="sm" onclick={onStop}>
            <Square class="size-4 mr-2" /> Stop Run
        </Button>
    {:else}
        <div class="flex">
            <Button
                variant="default"
                size="sm"
                class="rounded-r-none pr-3"
                disabled={isStarting}
                onclick={() => onRun(false)}
            >
                <Play class="size-4 mr-2" />
                {isStarting ? "Starting..." : "Run"}
            </Button>
            <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                    {#snippet child({ props })}
                        <Button
                            {...props}
                            variant="default"
                            size="sm"
                            class="rounded-l-none border-l border-primary-foreground/20 px-2"
                            disabled={isStarting}
                        >
                            <ChevronDown class="size-4" />
                        </Button>
                    {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end">
                    <DropdownMenu.Item onclick={() => onRun(true)}>
                        <Rocket class="size-4 mr-2 text-amber-500" />
                        <span class="font-medium">Force Run</span>
                    </DropdownMenu.Item>
                </DropdownMenu.Content>
            </DropdownMenu.Root>
        </div>
    {/if}
</div>
