<script lang="ts">
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import { Sun, Moon, Languages } from "lucide-svelte";
    import { toggleMode, mode } from "mode-watcher";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { t, locale } from "svelte-i18n";
</script>

<Sidebar.Menu>
    <Sidebar.MenuItem>
        <Sidebar.MenuButton onclick={toggleMode} title="Toggle Theme">
            {#if mode.current === "dark"}
                <Sun class="size-4" />
            {:else}
                <Moon class="size-4" />
            {/if}
            <span>Theme</span>
        </Sidebar.MenuButton>
    </Sidebar.MenuItem>

    <Sidebar.MenuItem>
        <DropdownMenu.Root>
            <DropdownMenu.Trigger>
                {#snippet child({ props })}
                    <Sidebar.MenuButton {...props} title={$t("language")}>
                        <Languages class="size-4" />
                        <span>{$t("language")} ({$locale?.toUpperCase()})</span>
                    </Sidebar.MenuButton>
                {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content side="top" align="start">
                {#each ["en", "zh-CN"] as tag}
                    <DropdownMenu.Item>
                        <button
                            onclick={() => ($locale = tag)}
                            class="w-full h-full text-left flex items-center"
                        >
                            {tag === "en" ? $t("english") : $t("chinese")}
                        </button>
                    </DropdownMenu.Item>
                {/each}
            </DropdownMenu.Content>
        </DropdownMenu.Root>
    </Sidebar.MenuItem>
</Sidebar.Menu>
