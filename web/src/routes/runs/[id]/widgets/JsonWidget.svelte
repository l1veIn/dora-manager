<script lang="ts">
    import { JSONEditor, Mode } from "svelte-jsoneditor";
    import "svelte-jsoneditor/themes/jse-theme-dark.css";

    let { asset } = $props<{
        asset: any;
        runId?: string;
    }>();

    // Catch parse errors if it's supposed to be JSON but fails
    let parsedData = $derived(() => {
        if (typeof asset.data === "string") {
            try {
                return JSON.parse(asset.data);
            } catch (e) {
                return { text: asset.data };
            }
        }
        return asset.data || {};
    });

    let content = $derived({
        json: parsedData(),
    });

    let isDarkMode = $state(false);

    $effect(() => {
        // Initial check
        isDarkMode = document.documentElement.classList.contains("dark");

        // Observe class changes on the html element
        const observer = new MutationObserver((mutations) => {
            mutations.forEach((mutation) => {
                if (mutation.attributeName === "class") {
                    isDarkMode =
                        document.documentElement.classList.contains("dark");
                }
            });
        });

        observer.observe(document.documentElement, { attributes: true });

        return () => {
            observer.disconnect();
        };
    });
</script>

<!-- We apply some CSS overrides to make it fit inside our IDE beautifully -->
<div
    class="w-full max-w-[45rem] rounded-md border shadow-sm overflow-x-auto overflow-y-hidden bg-background {isDarkMode
        ? 'jse-theme-dark'
        : ''}
    [&_.jse-main]:!border-none [&_.jse-main]:!rounded-md
"
    style="--jse-theme-color: hsl(var(--primary)); --jse-panel-background: hsl(var(--card)); --jse-background-color: transparent;"
>
    <JSONEditor
        {content}
        readOnly={true}
        mode={Mode.tree}
        navigationBar={false}
        mainMenuBar={false}
    />
</div>
