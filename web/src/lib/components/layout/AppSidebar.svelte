<script lang="ts">
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import {
        LayoutDashboard,
        PackageSearch,
        Waypoints,
        ActivitySquare,
        Settings,
    } from "lucide-svelte";
    import { page } from "$app/state";
    import AppFooter from "./AppFooter.svelte";

    const navItems = [
        { title: "Dashboard", url: "/", icon: LayoutDashboard },
        { title: "Nodes", url: "/nodes", icon: PackageSearch },
        { title: "Dataflows", url: "/dataflows", icon: Waypoints },
        { title: "Events", url: "/events", icon: ActivitySquare },
        { title: "Settings", url: "/settings", icon: Settings },
    ];
</script>

<Sidebar.Root variant="inset">
    <Sidebar.Header>
        <div class="px-4 py-2 font-bold text-lg text-primary tracking-tight">
            Dora Manager
        </div>
    </Sidebar.Header>

    <Sidebar.Content>
        <Sidebar.Group>
            <Sidebar.GroupLabel>Platform</Sidebar.GroupLabel>
            <Sidebar.GroupContent>
                <Sidebar.Menu>
                    {#each navItems as item (item.title)}
                        <Sidebar.MenuItem>
                            <Sidebar.MenuButton
                                isActive={page.url.pathname === item.url}
                            >
                                {#snippet child({ props })}
                                    <a href={item.url} {...props}>
                                        <item.icon class="size-4" />
                                        <span>{item.title}</span>
                                    </a>
                                {/snippet}
                            </Sidebar.MenuButton>
                        </Sidebar.MenuItem>
                    {/each}
                </Sidebar.Menu>
            </Sidebar.GroupContent>
        </Sidebar.Group>
    </Sidebar.Content>

    <Sidebar.Footer>
        <AppFooter />
    </Sidebar.Footer>
</Sidebar.Root>
