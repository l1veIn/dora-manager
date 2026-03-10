<script lang="ts">
    import { FileText, FolderOpen, Send } from "lucide-svelte";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Button } from "$lib/components/ui/button/index.js";

    let {
        mode = "file",
        id = undefined,
        value = $bindable(),
        placeholder = undefined,
        class: className = "",
        disabled = false,
        showConfirmBtn = false,
        confirming = false,
        onConfirm = undefined,
    } = $props<{
        mode?: "file" | "directory";
        id?: string;
        value?: string;
        placeholder?: string;
        class?: string;
        disabled?: boolean;
        showConfirmBtn?: boolean;
        confirming?: boolean;
        onConfirm?: (value: string) => void;
    }>();

    const isTauri =
        typeof window !== "undefined" && "__TAURI__" in window;

    let defaultPlaceholder = $derived(
        mode === "file" ? "Enter file path..." : "Enter directory path...",
    );

    function handleConfirm() {
        if (value?.trim() && onConfirm) onConfirm(value);
    }

    async function openPicker() {
        // TODO: Implement Tauri file/directory picker
    }
</script>

<div class="flex items-center gap-2 {className}">
    {#if mode === "file"}
        <FileText class="size-4 text-muted-foreground shrink-0" />
    {:else}
        <FolderOpen class="size-4 text-muted-foreground shrink-0" />
    {/if}
    <Input
        {id}
        {disabled}

        value={value ?? ""}
        oninput={(e) => value = (e.currentTarget as HTMLInputElement).value}
        placeholder={placeholder ?? defaultPlaceholder}
        class="flex-1 font-mono text-xs"
        onkeydown={(e: KeyboardEvent) => {
            if (e.key === 'Enter' && showConfirmBtn) {
                e.preventDefault();
                handleConfirm();
            }
        }}
    />
    {#if isTauri}
        <Button
            variant="outline"
            size="sm"
            class="shrink-0 h-8 px-2"
            {disabled}
            onclick={openPicker}
        >
            Browse
        </Button>
    {/if}
    {#if showConfirmBtn}
        <Button
            size="icon"
            variant="ghost"
            class="shrink-0 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
            disabled={disabled || confirming || !value?.trim()}
            onclick={handleConfirm}
            title="Send"
        >
            {#if confirming}
                <div class="h-3.5 w-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
            {:else}
                <Send class="h-3.5 w-3.5" />
            {/if}
        </Button>
    {/if}
</div>
