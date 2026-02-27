<script lang="ts">
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { useStatus } from "$lib/stores/status.svelte";

  const store = useStatus();
</script>

<header
  class="flex h-16 shrink-0 items-center justify-between gap-2 border-b px-4"
>
  <div class="flex items-center gap-2">
    <Sidebar.Trigger class="-ml-1" />
    <div class="font-bold text-lg mr-2 font-mono">dm</div>
    <Badge variant="secondary" class="font-mono text-xs font-normal"
      >{store.status?.active_version || "..."}</Badge
    >
  </div>
  <div class="flex items-center gap-2">
    <div
      class="h-2 w-2 rounded-full {store.status?.runtime_running
        ? 'bg-green-500'
        : 'bg-slate-400'}"
      title={store.status?.runtime_running ? "Dora Running" : "Dora Stopped"}
    ></div>
    <span class="text-sm text-muted-foreground mr-2 font-mono"
      >{store.status?.runtime_running ? "Running" : "Stopped"}</span
    >
  </div>
</header>
