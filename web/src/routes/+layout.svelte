<script lang="ts">
	import { ParaglideJS } from "@inlang/paraglide-sveltekit";
	import { i18n } from "$lib/i18n";
	import { ModeWatcher } from "mode-watcher";
	import "../app.css";

	import * as Sidebar from "$lib/components/ui/sidebar/index.js";
	import AppSidebar from "$lib/components/layout/AppSidebar.svelte";
	import AppHeader from "$lib/components/layout/AppHeader.svelte";
	import { Toaster } from "$lib/components/ui/sonner/index.js";

	let { children } = $props();
</script>

<ParaglideJS {i18n}>
	<ModeWatcher />
	<Toaster />

	<Sidebar.Provider>
		<AppSidebar />
		<main
			class="w-full h-screen overflow-hidden flex flex-col items-stretch"
		>
			<AppHeader />
			<div class="flex-1 overflow-auto bg-background text-foreground">
				{@render children()}
			</div>
		</main>
	</Sidebar.Provider>
</ParaglideJS>
