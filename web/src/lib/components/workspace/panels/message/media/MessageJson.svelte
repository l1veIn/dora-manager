<script lang="ts">
    import { JSONEditor, Mode } from "svelte-jsoneditor";
    import "svelte-jsoneditor/themes/jse-theme-dark.css";

    let { content } = $props<{ content: any }>();

    let parsedData = $derived(() => {
        if (typeof content === "string") {
            try { return JSON.parse(content); } catch (e) { return { text: content }; }
        }
        return content || {};
    });

    let svelteContent = $derived({ json: parsedData() });
    let isDarkMode = $state(false);

    $effect(() => {
        isDarkMode = document.documentElement.classList.contains("dark");
        const observer = new MutationObserver((mutations) => {
            mutations.forEach((m) => {
                if (m.attributeName === "class") {
                    isDarkMode = document.documentElement.classList.contains("dark");
                }
            });
        });
        observer.observe(document.documentElement, { attributes: true });
        return () => observer.disconnect();
    });
</script>

<div class="w-full max-w-[500px] min-w-[280px] rounded-md border shadow-sm overflow-x-auto overflow-y-hidden bg-background {isDarkMode ? 'jse-theme-dark' : ''} [&_.jse-main]:!border-none [&_.jse-main]:!rounded-md"
    style="--jse-theme-color: hsl(var(--primary)); --jse-panel-background: hsl(var(--card)); --jse-background-color: transparent;">
    <JSONEditor content={svelteContent} readOnly={true} mode={Mode.tree} navigationBar={false} mainMenuBar={false} />
</div>
