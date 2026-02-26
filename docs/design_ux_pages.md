# dm Web — Page UX Specifications

## Overview

This document describes the user experience for each page in the dm web interface.
Each section covers: purpose, API dependencies, component layout, interactions, and states.

---

## 1. Dashboard (`/`)

### Purpose
At-a-glance system health overview. The user should instantly know: is Dora running? What version? How many nodes are installed?

### API Dependencies
| API | Usage |
|---|---|
| `GET /api/status` | Dora runtime status (up/down) |
| `GET /api/doctor` | Environment health checks |
| `GET /api/versions` | Installed & active Dora versions |
| `GET /api/nodes` | Installed node count |

### Layout
```
┌─ Dashboard ─────────────────────────────────┐
│                                             │
│  ┌─ Status Card ──────┐  ┌─ Version Card ─┐│
│  │ ● Dora Running     │  │ Active: 0.4.1  ││
│  │   Coordinator: ✓   │  │ Installed: 3   ││
│  │   Daemon: ✓        │  │                ││
│  │   [Stop] button    │  │ [Switch] btn   ││
│  └────────────────────┘  └────────────────┘│
│                                             │
│  ┌─ Health Card ──────────────────────────┐ │
│  │ ✓ Python 3.11  ✓ uv  ✓ dora  ✓ PATH  │ │
│  │ ✕ Node XYZ missing dependency         │ │
│  └────────────────────────────────────────┘ │
│                                             │
│  ┌─ Quick Stats ──────────────────────────┐ │
│  │ Nodes: 5 installed  │  Events: 1,204   │ │
│  └────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

### Interactions
- **[Stop]/[Start] button**: Calls `POST /api/down` or `POST /api/up`, shows toast notification
- **[Switch] button**: Opens dialog → calls `POST /api/use` with selected version
- All cards auto-refresh every 10s via `$effect` + `setInterval`

### States
- **Loading**: Skeleton cards (shadcn Skeleton component)
- **Error**: Red alert banner with retry button
- **Dora Not Installed**: CTA card → "Run `dm setup` or click Install"

---

## 2. Nodes (`/nodes`)

### Purpose
Browse available nodes from the registry, manage installed nodes. Similar to an "app store" for Dora nodes.

### API Dependencies
| API | Usage |
|---|---|
| `GET /api/registry` | Available nodes from remote registry |
| `GET /api/nodes` | Locally installed nodes |
| `GET /api/nodes/{id}` | Single node detail |
| `POST /api/nodes/install` | Install a node |
| `POST /api/nodes/uninstall` | Uninstall a node |

### Layout
```
┌─ Nodes ─────────────────────────────────────────┐
│                                                 │
│  [Search input]     Tabs: [Installed] [Registry]│
│                                                 │
│  ── Installed Tab ──                            │
│  ┌─────────────────────────────────────────┐    │
│  │ opencv-video-capture      v0.1.0        │    │
│  │ Python · ~/.dm/nodes/opencv-video-cap…  │    │
│  │                          [Uninstall] ▼  │    │
│  ├─────────────────────────────────────────┤    │
│  │ opencv-plot               v0.1.0        │    │
│  │ Python · ~/.dm/nodes/opencv-plot        │    │
│  │                          [Uninstall] ▼  │    │
│  └─────────────────────────────────────────┘    │
│                                                 │
│  ── Registry Tab ──                             │
│  ┌─────────────────────────────────────────┐    │
│  │ webcam-capture            ★ 12          │    │
│  │ Captures webcam frames as dora arrows   │    │
│  │ [Install]                               │    │
│  ├─────────────────────────────────────────┤    │
│  │ llm-openai               ★ 8           │    │
│  │ OpenAI GPT integration node             │    │
│  │ [Installed ✓] (disabled)                │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### Interactions
- **Search**: Client-side filter on node name/description
- **[Install]**: Shows progress indicator → toast on success
- **[Uninstall]**: Confirmation dialog → delete → refresh list
- Registry tab items already installed show a grayed "Installed ✓" badge

### States
- **Loading**: Skeleton list items
- **Empty installed**: Illustration + "No nodes installed" + CTA
- **Registry fetch error**: Alert with retry

---

## 3. Editor (`/editor`)

### Purpose
Create, edit, and execute Dora dataflow YAML files. This is the core creative workspace.

### API Dependencies
| API | Usage |
|---|---|
| `POST /api/dataflow/run` | Execute the YAML |
| `POST /api/dataflow/stop` | Stop running dataflow |
| `GET /api/nodes` | Autocomplete node names |

### Layout
```
┌─ Editor ────────────────────────────────────────┐
│                                                 │
│  Toolbar: [▶ Run] [■ Stop] [📋 Template ▼]      │
│                                                 │
│  ┌─────────────────────────────────────────┐    │
│  │ (CodeMirror 6 YAML editor)              │    │
│  │                                         │    │
│  │ nodes:                                  │    │
│  │   - id: webcam                          │    │
│  │     operator:                           │    │
│  │       python: webcam-capture            │    │
│  │                                         │    │
│  │   - id: plot                            │    │
│  │     operator:                           │    │
│  │       python: opencv-plot               │    │
│  │     inputs:                             │    │
│  │       image: webcam/image               │    │
│  │                                         │    │
│  └─────────────────────────────────────────┘    │
│                                                 │
│  ┌─ Output Panel (collapsible) ───────────┐    │
│  │ [14:32:01] Started dataflow df_abc123   │    │
│  │ [14:32:02] Node webcam spawned (pid 42) │    │
│  │ [14:32:03] Node plot spawned (pid 43)   │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### Interactions
- **[▶ Run]**: Sends editor content to `POST /api/dataflow/run`, switches to "running" state
- **[■ Stop]**: Calls `POST /api/dataflow/stop`
- **[Template ▼]**: Dropdown with example YAML templates (quickstart, multi-node, etc.)
- **Output Panel**: Shows events from `GET /api/events?source=dataflow&limit=50`, auto-scrolls
- **Editor**: CodeMirror 6 with YAML syntax highlighting, line numbers, auto-indent

### States
- **Idle**: Run button enabled, Stop disabled
- **Running**: Run button disabled (or shows ↻), Stop enabled, output panel auto-opens
- **Error**: Red toast with error message from API

### Recommended Package
- `svelte-codemirror-editor` + `@codemirror/lang-yaml`

---

## 4. Events (`/events`)

### Purpose
Real-time observability panel. View, search, filter, and export all system events (logs, analytics, process mining data).

### API Dependencies
| API | Usage |
|---|---|
| `GET /api/events` | Query events with filters |
| `GET /api/events/export` | Export XES XML |

### Layout
```
┌─ Events ────────────────────────────────────────┐
│                                                 │
│  Filters: [Source ▼] [Level ▼] [Search...]      │
│           [Date range picker]    [Export XES]    │
│                                                 │
│  ┌─ Event Table ──────────────────────────────┐ │
│  │ Time       │ Source  │ Activity      │ Msg  │ │
│  │────────────┼────────┼──────────────┼──────│ │
│  │ 14:32:01   │ core   │ node.install  │ ...  │ │
│  │ 14:32:00   │ server │ http.request  │ ...  │ │
│  │ 14:31:58   │ datafl │ node.spawn    │ ...  │ │
│  │ 14:31:55   │ frontn │ ui.click      │ ...  │ │
│  └────────────────────────────────────────────┘ │
│                                                 │
│  [Load more] or infinite scroll                 │
└─────────────────────────────────────────────────┘
```

### Interactions
- **Filters**: Each filter immediately updates the table via API call
- **Source dropdown**: core / dataflow / server / frontend / ci
- **Level dropdown**: trace / debug / info / warn / error
- **Search**: Fuzzy search in `activity` field
- **Row click**: Expands to show full `attributes` JSON in a Sheet/Drawer
- **[Export XES]**: Downloads XML file via `GET /api/events/export?{current_filters}`

### shadcn Components
- `Table` for the event list
- `Select` for source/level filters
- `Input` for search
- `Badge` for source/level tags (color-coded)
- `Sheet` for event detail view

---

## 5. Settings (`/settings`)

### Purpose
Manage Dora versions, dm configuration, and environment settings.

### API Dependencies
| API | Usage |
|---|---|
| `GET /api/config` | Current config |
| `POST /api/config` | Update config |
| `GET /api/versions` | Version list |
| `POST /api/install` | Install a version |
| `POST /api/uninstall` | Remove a version |
| `POST /api/use` | Switch active version |

### Layout
```
┌─ Settings ──────────────────────────────────────┐
│                                                 │
│  ─── Dora Versions ───                          │
│                                                 │
│  ┌─────────────────────────────────────────┐    │
│  │ 0.4.1  ● active    [Uninstall]          │    │
│  │ 0.3.9              [Use] [Uninstall]    │    │
│  └─────────────────────────────────────────┘    │
│                                                 │
│  [Install Version ▼]                            │
│                                                 │
│  ─── Configuration ───                          │
│                                                 │
│  Active Version:  [0.4.1 ▼]                     │
│  DM Home:         ~/.dm  (read-only)            │
│                                                 │
│  ─── Environment ───                            │
│                                                 │
│  Python: 3.11.5  ✓                              │
│  uv: 0.5.1  ✓                                   │
│  Dora binary: ~/.dm/versions/0.4.1/dora  ✓      │
│                                                 │
│  ─── About ───                                  │
│                                                 │
│  dm version: 0.1.0                              │
│  GitHub: github.com/l1veIn/dora-manager          │
└─────────────────────────────────────────────────┘
```

### Interactions
- **[Use]**: Switches active version → refreshes page
- **[Uninstall]**: Confirmation dialog
- **[Install Version]**: Dropdown with available versions from API
- All settings changes show a toast notification

---

## Shared Patterns

### Loading States
All data-fetching pages use shadcn `Skeleton` components in the exact same layout as the loaded state.

### Error Handling
Use shadcn `Alert` component (variant=destructive) with a retry button. Non-critical errors use toast notifications.

### Toasts
Use shadcn `Sonner` (toast) for all operation feedback:
- Success: green check + message (auto-dismiss 3s)
- Error: red X + message (sticky until dismissed)

### Data Fetching Pattern (Svelte 5)
```svelte
<script lang="ts">
  import { get } from '$lib/api';

  let data = $state<SomeType | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  $effect(() => {
    get<SomeType>('/some-endpoint')
      .then(d => { data = d; })
      .catch(e => { error = e.message; })
      .finally(() => { loading = false; });
  });
</script>
```
