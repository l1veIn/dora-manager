<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar/index.js';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import { get } from '$lib/api';
  import { onMount } from 'svelte';

  let version = $state<string>('...');
  let isRunning = $state<boolean>(false);

  async function fetchStatus() {
    try {
      const statusRes: any = await get('/status');
      if (statusRes.dora_daemon_status === 'running') {
        isRunning = true;
      } else {
        isRunning = false;
      }
    } catch (e) {
      isRunning = false;
    }

    try {
      const vRes: any = await get('/versions');
      version = vRes.active_version || 'N/A';
    } catch (e) {
      version = 'Error';
    }
  }

  onMount(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 10000);
    return () => clearInterval(interval);
  });
</script>

<header class="flex h-16 shrink-0 items-center justify-between gap-2 border-b px-4">
  <div class="flex items-center gap-2">
    <Sidebar.Trigger class="-ml-1" />
    <div class="font-bold text-lg mr-2 font-mono">dm</div>
    <Badge variant="secondary" class="font-mono text-xs font-normal">{version}</Badge>
  </div>
  <div class="flex items-center gap-2">
    <div
      class="h-2 w-2 rounded-full {isRunning ? 'bg-green-500' : 'bg-slate-400'}"
      title={isRunning ? 'Dora Running' : 'Dora Stopped'}
    ></div>
    <span class="text-sm text-muted-foreground mr-2 font-mono">{isRunning ? 'Running' : 'Stopped'}</span>
  </div>
</header>
