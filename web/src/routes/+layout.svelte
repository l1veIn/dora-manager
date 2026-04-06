<script lang="ts">
	import "$lib/i18n";
	import { ModeWatcher } from "mode-watcher";
	import { browser } from "$app/environment";
	import "../app.css";

	import * as Sidebar from "$lib/components/ui/sidebar/index.js";
	import AppSidebar from "$lib/components/layout/AppSidebar.svelte";
	import AppHeader from "$lib/components/layout/AppHeader.svelte";
	import { Toaster } from "$lib/components/ui/sonner/index.js";
	import { page } from "$app/state";

	let { children } = $props();

	let isEditorRoute = $derived(page.url?.pathname?.endsWith('/editor') ?? false);
	let appSidebarOpen = $state(true);

	$effect(() => {
		if (!browser) return;
		const saved = localStorage.getItem("dm-app-sidebar-open");
		if (saved !== null) {
			appSidebarOpen = saved === "true";
		}
	});

	$effect(() => {
		if (!browser) return;
		localStorage.setItem("dm-app-sidebar-open", String(appSidebarOpen));
	});
</script>

<ModeWatcher />
<Toaster />

{#if isEditorRoute}
	<div class="h-screen w-screen overflow-hidden bg-background text-foreground">
		{@render children()}
	</div>
{:else}
	<Sidebar.Provider bind:open={appSidebarOpen}>
		<AppSidebar />
		<main
			class="flex-1 w-full h-screen max-h-screen overflow-hidden flex flex-col min-w-0"
		>
			<AppHeader />
			<div
				class="flex-1 min-h-0 overflow-auto bg-background text-foreground relative"
			>
				{@render children()}
			</div>
		</main>
	</Sidebar.Provider>
{/if}
