<script lang="ts">
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import {
        LayoutDashboard,
        PackageSearch,
        Wrench,
        Waypoints,
        History,
        ActivitySquare,
        Settings,
    } from "lucide-svelte";
    import { page } from "$app/state";
    import AppFooter from "./AppFooter.svelte";

    const navItems = [
        { title: "Dashboard", url: "/", icon: LayoutDashboard },
        { title: "Nodes", url: "/nodes", icon: PackageSearch },
        { title: "Services", url: "/services", icon: Wrench },
        { title: "Dataflows", url: "/dataflows", icon: Waypoints },
        { title: "Runs", url: "/runs", icon: History },
        { title: "Events", url: "/events", icon: ActivitySquare },
        { title: "Settings", url: "/settings", icon: Settings },
    ];
</script>

<Sidebar.Root variant="inset" class="border-r border-sidebar-border/80">
    <Sidebar.Header>
        <div class="flex items-center justify-between gap-2 px-4 py-2">
            <div class="font-bold text-lg text-primary tracking-tight">
                Dora Manager
            </div>
            <Sidebar.Trigger class="size-7" />
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

    <Sidebar.Rail />
</Sidebar.Root>
