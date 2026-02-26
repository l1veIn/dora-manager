<script lang="ts">
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import { Sun, Moon, Languages } from "lucide-svelte";
    import { toggleMode, mode } from "mode-watcher";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import {
        setLanguageTag,
        languageTag,
        availableLanguageTags,
    } from "$lib/paraglide/runtime";
    import { i18n } from "$lib/i18n";
    import { page } from "$app/state";
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
                    <Sidebar.MenuButton {...props} title="Language">
                        <Languages class="size-4" />
                        <span>Language ({languageTag()})</span>
                    </Sidebar.MenuButton>
                {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content side="top" align="start">
                {#each availableLanguageTags as tag}
                    <DropdownMenu.Item>
                        <a
                            href={i18n.resolveRoute(page.url.pathname, tag)}
                            class="w-full h-full flex items-center"
                        >
                            {tag === "en" ? "English" : "中文"}
                        </a>
                    </DropdownMenu.Item>
                {/each}
            </DropdownMenu.Content>
        </DropdownMenu.Root>
    </Sidebar.MenuItem>
</Sidebar.Menu>
