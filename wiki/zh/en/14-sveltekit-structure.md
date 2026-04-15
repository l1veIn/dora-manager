Dora Manager's frontend is a single-page application (SPA) based on **SvelteKit + Svelte 5**, communicating with the Rust backend (`dm-server`) through lightweight HTTP clients and WebSocket. This document focuses on the frontend's engineering skeleton, communication mechanisms, state management paradigms, and route organization — the "base infrastructure" supporting all page functionality.

Sources: [svelte.config.js](https://github.com/l1veIn/dora-manager/blob/master/web/svelte.config.js#L1-L15), [vite.config.ts](https://github.com/l1veIn/dora-manager/blob/master/web/vite.config.ts#L1-L16)

## Engineering Skeleton: SPA-Mode SvelteKit Configuration

The project uses `@sveltejs/adapter-static` with `fallback: 'index.html'` set, meaning the build output is a set of pure static files embedded and served by the backend through `rust_embed`. The rendering strategy explicitly disables SSR and prerendering:

```typescript
export const prerender = false;  // Don't prerender any pages
export const ssr = false;        // Fully client-side rendering
```

This choice determines the entire frontend's runtime model: **all page logic executes on the browser side**, with no server-side data prefetching (load functions); API data relies entirely on client-side `fetch` calls.

Sources: [+layout.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+layout.ts#L1-L3), [svelte.config.js](https://github.com/l1veIn/dora-manager/blob/master/web/svelte.config.js#L1-L13)

### Core Tech Stack

| Layer | Technology Choice | Purpose |
|-------|------------------|---------|
| Framework | SvelteKit 2 + Svelte 5 | Routing, compilation optimization, Runes reactivity |
| Build Tool | Vite 7 | Dev server, HMR, production build |
| UI Component Library | shadcn-svelte (bits-ui) | Unstyled primitive components (Dialog, Select, Sheet, etc.) |
| Styles | Tailwind CSS v4 + tailwind-merge | Atomic CSS, dark mode |
| Graph Editor | @xyflow/svelte | Dataflow visualization canvas |
| Grid Layout | GridStack 12 | Run workspace panel drag/resize |
| Code Editor | svelte-codemirror-editor | YAML/JSON editing |
| Internationalization | svelte-i18n | Chinese-English bilingual |
| Icons | lucide-svelte | Unified icon set |

Sources: [package.json](https://github.com/l1veIn/dora-manager/blob/master/web/package.json#L1-L66), [components.json](https://github.com/l1veIn/dora-manager/blob/master/web/components.json#L1-L16)

## Directory Structure Panorama

```
web/src/
├── app.css / app.d.ts / app.html    # SvelteKit application entry
├── lib/                              # Shared library code
│   ├── api.ts                        # ← HTTP communication core (four functions)
│   ├── i18n.ts                       # Internationalization initialization
│   ├── utils.ts                      # cn() style merge utility
│   ├── index.ts                      # $lib alias entry
│   ├── stores/
│   │   └── status.svelte.ts          # Global state: runtime / doctor / nodes
│   ├── locales/                      # Translation files
│   ├── hooks/                        # Client-side hooks
│   ├── components/
│   │   ├── ui/                       # shadcn-svelte atomic components (30+ directories)
│   │   ├── layout/                   # AppHeader / AppSidebar / AppFooter
│   │   ├── runs/                     # RunStatusBadge / RecentRunCard
│   │   ├── dataflows/                # DataflowRunActions
│   │   └── workspace/               # GridStack workspace system
│   │       ├── Workspace.svelte      # Grid container + GridStack integration
│   │       ├── types.ts              # WorkspaceGridItem type definitions
│   │       ├── widgets/              # RootWidgetWrapper (panel shell)
│   │       └── panels/              # Panel registry + five panel implementations
│   │           ├── registry.ts       # Panel registry center
│   │           ├── types.ts          # PanelContext / PanelDefinition types
│   │           ├── message/          # Message panel + cursor pagination state machine
│   │           ├── input/            # Input control panel
│   │           ├── chart/            # Chart panel
│   │           ├── video/            # Video/HLS panel
│   │           └── terminal/         # Terminal log panel
│   └── assets/
├── routes/                           # SvelteKit file-system routing
│   ├── +layout.svelte                # Global layout (Sidebar + Header + ModeWatcher)
│   ├── +layout.ts                    # Disable SSR/prerendering
│   ├── +page.svelte                  # Dashboard homepage
│   ├── dataflows/
│   │   ├── +page.svelte              # Dataflow list
│   │   └── [id]/
│   │       ├── +page.svelte          # Dataflow details (Tabs: Graph/YAML/Meta/History)
│   │       ├── editor/+page.svelte   # Full-screen graph editor
│   │       └── components/           # GraphEditorTab / YamlEditorTab / graph/
│   ├── runs/
│   │   ├── +page.svelte              # Run list (pagination, search, filter)
│   │   └── [id]/
│   │       ├── +page.svelte          # Run details (Workspace + sidebar)
│   │       ├── InteractionPane.svelte # Interaction panel (Display + Input node bridging)
│   │       ├── RunLogViewer.svelte   # Terminal log (tail mode polling)
│   │       └── graph/                # RuntimeGraphView (WebSocket-driven)
│   ├── nodes/
│   │   ├── +page.svelte              # Node list
│   │   └── [id]/+page.svelte         # Node details
│   ├── events/+page.svelte           # Event log
│   └── settings/+page.svelte         # Settings (versions/media/config)
```

Sources: [get_dir_structure](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/registry.ts#L1-L80), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/dataflows/+page.svelte#L1-L52)

## API Communication Layer: Minimalist HTTP Client

The entire frontend's HTTP communication is concentrated in a 33-line module — `$lib/api.ts`. It exports four generic functions, unifying error handling and response parsing for all backend interactions:

```typescript
export const API_BASE = '/api';

export async function get<T>(path: string): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`);
    if (!res.ok) throw new Error(await res.text());
    return res.json();
}
```

`post` and `del` follow the same pattern: setting `Content-Type: application/json` and throwing exceptions containing server error text on non-200 responses. `getText` is the plain text variant of `get`, used for obtaining YAML source code and logs.

Sources: [api.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/api.ts#L1-L33)

### Development Proxy and Production Communication

During development, Vite is configured with proxy rules to forward `/api` requests to `http://127.0.0.1:3210` (dm-server's default port), with WebSocket proxying enabled (`ws: true`):

```typescript
server: {
    proxy: {
        '/api': {
            target: 'http://127.0.0.1:3210',
            changeOrigin: true,
            ws: true    // Proxy WebSocket upgrade requests
        }
    }
}
```

In production, static files are directly embedded and served by dm-server through `rust_embed`, with `/api` paths naturally routing to the same process's HTTP handlers — no cross-origin issues, no CORS configuration needed.

Sources: [vite.config.ts](https://github.com/l1veIn/dora-manager/blob/master/web/vite.config.ts#L8-L15)

### API Endpoint Call Panorama

The following table summarizes all HTTP requests made by frontend pages through `api.ts`:

| Module | HTTP Method | Endpoint Path | Purpose |
|--------|------------|---------------|---------|
| Dashboard | GET | `/status`, `/doctor`, `/nodes` | Runtime health status |
| Dashboard | GET | `/runs/active?metrics=true`, `/runs?limit=100` | Active runs and history list |
| Dataflows | GET | `/dataflows`, `/media/status` | Dataflow list and media status |
| Dataflows | POST | `/dataflows/{name}`, `/dataflows/{name}/delete` | Create/delete dataflow |
| Dataflows | POST | `/runs/start` | Start dataflow run |
| Dataflow Detail | GET | `/dataflows/{name}` | Get YAML + metadata |
| Dataflow Editor | POST | `/dataflows/{name}`, `/dataflows/{name}/view` | Save YAML and view layout |
| Runs List | GET | `/runs?limit=&offset=&status=&search=` | Paginated + filtered query |
| Runs List | POST | `/runs/delete` | Batch delete runs |
| Run Detail | GET | `/runs/{id}`, `/runs/{id}/messages/snapshots` | Run details + message snapshots |
| Run Detail | GET | `/runs/{id}/messages?tag=input&limit=5000` | Input value initialization |
| Run Detail | GET | `/runs/{id}/messages?tag=input&after_seq=` | Incremental input value fetch |
| Run Detail | POST | `/runs/{id}/messages`, `/runs/{id}/stop` | Send message / stop run |
| Run Detail | GET | `/runs/{id}/dataflow`, `/runs/{id}/transpiled`, `/runs/{id}/view` | View source/transpilation |
| Run Logs | GET | `/runs/{id}/logs/{nodeId}`, `/runs/{id}/logs/{nodeId}/tail?offset=` | Full/incremental logs |
| Nodes | GET | `/nodes` | Installed node list |
| Nodes | POST | `/nodes/download`, `/nodes/install`, `/nodes/uninstall` | Node lifecycle management |
| Events | GET | `/events/count`, `/events?limit=&offset=` | Event count and list |
| Settings | GET | `/config`, `/versions`, `/doctor`, `/media/status` | Configuration and environment info |
| Settings | POST | `/config`, `/install`, `/use`, `/uninstall`, `/media/install` | Version and media management |

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+page.svelte#L32-L83), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/+page.svelte#L64-L85), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/dataflows/+page.svelte#L38-L56), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L175-L315)

## Real-Time Communication: WebSocket and Polling Dual-Track Mechanism

The frontend uses two complementary real-time data fetching strategies:

### WebSocket: Run Message Flow

The run details page establishes a **run-scoped WebSocket connection** in `onMount`:

```typescript
const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
const socket = new WebSocket(
    `${protocol}//${window.location.host}/api/runs/${runId}/messages/ws`
);
```

After the connection is established, whenever the backend pushes a notification (`socket.onmessage`), the frontend does two things:
1. Call `fetchSnapshots()` to refresh interaction node (dm-display) state snapshots
2. If the notification tag is `input`, call `fetchNewInputValues()` to get incremental input values

After WebSocket disconnection, it automatically reconnects after 1 second (`scheduleMessageSocketReconnect`). The connection is cleaned up when the component is destroyed.

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L321-L392)

### Polling: Status Refresh

For data that does not require millisecond-level real-time updates, the frontend uses `setInterval` polling:

| Scenario | Interval | Trigger Condition |
|----------|----------|-------------------|
| Dashboard run overview | 3 seconds | When page is visible |
| Run details (running) | 3 seconds | `run.status === "running"` |
| Node logs (tail mode) | 2 seconds | `isRunActive && nodeId` |
| Post-stop status confirmation | 1 second | Up to 10 times |

The Dashboard page additionally listens for `visibilitychange` events, immediately refreshing data when the tab becomes visible again, avoiding stale information display.

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L375-L392), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+page.svelte#L85-L103), [RunLogViewer.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/RunLogViewer.svelte#L76-L88)

## Data Fetching Patterns and Deduplication Strategy

### Promise Deduplication (Deduplication-in-flight)

The run details page uses "in-flight Promise reuse" in multiple places to avoid data races from concurrent requests:

```typescript
let snapshotRefreshInFlight: Promise<void> | null = null;

async function fetchSnapshots() {
    if (snapshotRefreshInFlight) return snapshotRefreshInFlight;  // Reuse in-flight request
    snapshotRefreshInFlight = (async () => {
        // ... actual fetch logic
    })();
    return snapshotRefreshInFlight;
}
```

`fetchInputValues` and `fetchNewInputValues` follow the same pattern, ensuring that even if WebSocket notifications and timers simultaneously trigger `fetchSnapshots()`, only one HTTP request is sent.

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L218-L262)

### Cursor Pagination: Message History State Machine

`message-state.svelte.ts` exports `createMessageHistoryState`, a **bidirectional pagination state machine based on sequence-number cursors**, specifically designed for browsing run message timelines:

```
     ← Load older messages    [oldestSeq ... messages ... newestSeq]    Load newer messages →
         before_seq          ↑ Cursor pagination base ↑           after_seq
```

Three core methods:
- **`loadInitial`**: Load the latest 50 messages in reverse order (`desc: true`), used for initializing the view
- **`loadNew`**: Use `after_seq: newestSeq` to get new messages after the cursor, for real-time appending
- **`loadOld`**: Use `before_seq: oldestSeq` + `desc: true` to get earlier history, supporting infinite upward scrolling

Each method has built-in `fetching` / `fetchingOld` mutexes to prevent concurrent requests. The message panel (`MessagePanel.svelte`) triggers `loadNew` by monitoring `context.refreshToken` changes, implementing WebSocket-driven incremental updates.

Sources: [message-state.svelte.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/message/message-state.svelte.ts#L20-L119), [MessagePanel.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/message/MessagePanel.svelte#L113-L125)

## Global State Management

### Global Runtime State

`$lib/stores/status.svelte.ts` leverages Svelte 5's `$state` rune to create a **module-level singleton state**, encapsulating three reactive variables: `status` (runtime state), `doctor` (environment diagnostics), and `nodes` (node list):

```typescript
let status = $state<any>(null);
let doctor = $state<any>(null);
let nodes = $state<any[]>([]);
let loading = $state(true);

async function refresh(showSkeleton = false) {
    [status, doctor, nodes] = await Promise.all([
        get('/status').catch(() => null),
        get('/doctor').catch(() => null),
        get('/nodes').catch(() => []),
    ]);
}

export function useStatus() {
    return {
        get status() { return status; },
        get doctor() { return doctor; },
        // ... expose read-only accessors + refresh method
    };
}
```

`useStatus()` returns an object containing getters — external components can read the latest values and call `refresh()`, but cannot directly assign. Dashboard and AppHeader both consume this global state to display runtime version numbers and environment health.

Sources: [status.svelte.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/stores/status.svelte.ts#L1-L35), [AppHeader.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/layout/AppHeader.svelte#L1-L20)

### Page-Level Local State

In addition to global state, each page component uses Svelte 5's `$state` / `$derived` / `$effect` runes to manage its own local state. Typical pattern:

```typescript
let runs = $state<any[]>([]);           // Data
let loading = $state(true);             // Loading state
let currentPage = $state(1);            // UI state

async function fetchRuns() {             // Data fetching function
    loading = true;
    try { runs = (await get(`/runs?...`)).runs; }
    finally { loading = false; }
}

onMount(() => { fetchRuns(); });        // Load on mount
```

This "data as state" pattern does not introduce additional state management libraries (like Redux or Zustand), relying on Svelte 5's fine-grained reactivity system to automatically track dependencies and efficiently update the DOM.

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/+page.svelte#L22-L85), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/nodes/+page.svelte#L15-L90)

## Route Organization and Layout Strategy

### File-System Routing

SvelteKit's file-system routing organizes the application into 6 top-level pages:

```
/                    → Dashboard (overview + frequent dataflows + active runs)
/dataflows           → Dataflow list (search + create + run)
/dataflows/[id]      → Dataflow details (Graph/YAML/Meta/History four tabs)
/dataflows/[id]/editor → Full-screen graph editor (SvelteFlow canvas)
/runs                → Run list (paginated table + search + filter + batch delete)
/runs/[id]           → Run workspace (Workspace + sidebar + interaction panel)
/nodes               → Node list (install/download/uninstall operations)
/nodes/[id]          → Node details
/events              → Event log (filter + XES export)
/settings            → Settings (version management + media configuration)
```

Sources: [AppSidebar.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/layout/AppSidebar.svelte#L14-L21)

### Dual Layout Mode

The root layout (`+layout.svelte`) selects different shell structures based on the current route:

- **Regular routes**: `Sidebar.Provider` + `AppSidebar` + `AppHeader` + main content area, forming a classic sidebar navigation layout
- **Editor routes** (`/dataflows/[id]/editor`): Completely hides sidebar and header, providing full-screen canvas space

```svelte
{#if isEditorRoute}
    <div class="h-screen w-screen overflow-hidden">
        {@render children()}
    </div>
{:else}
    <Sidebar.Provider bind:open={appSidebarOpen}>
        <AppSidebar />
        <main><!-- AppHeader + children --></main>
    </Sidebar.Provider>
{/if}
```

The sidebar's expand/collapse state is persisted to `localStorage`, and each run details page also has its own independent sidebar state key.

Sources: [+layout.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+layout.svelte#L1-L54)

## Panel Registry and Workspace System

The core of the run details page is the **Workspace** — a dynamic panel system based on GridStack. Panels are managed through a registry (`registry.ts`), with each panel type declaring its metadata:

```typescript
export const panelRegistry: Record<PanelKind, PanelDefinition> = {
    message:  { kind: "message",  sourceMode: "history",   supportedTags: "*",       ... },
    input:    { kind: "input",    sourceMode: "snapshot",   supportedTags: ["widgets"], ... },
    chart:    { kind: "chart",    sourceMode: "snapshot",   supportedTags: ["chart"],   ... },
    video:    { kind: "video",    sourceMode: "snapshot",   supportedTags: ["stream"],  ... },
    terminal: { kind: "terminal", sourceMode: "external",   supportedTags: [],          ... },
};
```

**`sourceMode`** determines how panels fetch data:

| Mode | Data Fetching Method | Representative Panel |
|------|---------------------|---------------------|
| `history` | API requests via `createMessageHistoryState`, cursor pagination | Message |
| `snapshot` | Filtered from `context.snapshots`, data refreshed by parent component via WebSocket | Input, Chart |
| `external` | Self-managed independent polling/data source | Terminal (log tail) |

`PanelContext` serves as the unified interface for all panels, passing down `runId`, `snapshots`, `inputValues`, `nodes`, `emitMessage` and other data and methods, so panels need not concern themselves with data sources.

Sources: [registry.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/registry.ts#L1-L80), [types.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/types.ts#L1-L41), [Workspace.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/Workspace.svelte#L1-L175)

## Interaction Panel: Bridging Frontend with dm-display/dm-input

`InteractionPane.svelte` is the core component for handling **interaction nodes** in the run scope. It manages two types of data simultaneously:

- **Streams (display streams)**: Filter `snapshots` for entries with `kind` as display, selecting rendering method based on the `render` field (`text`/`json`/`markdown`/`image`/`audio`/`video`). For non-inline artifact files, content is fetched via `/api/runs/{runId}/artifacts/{file}`.
- **Inputs (input bindings)**: Extract `widgets` configuration from `inputs` for each input node, dynamically rendering controls (text boxes, sliders, switches, checkboxes, dropdown selects). After user interaction, `emit()` is called to encode the value as a message sent to the backend.

Sources: [InteractionPane.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/InteractionPane.svelte#L1-L156)

## Persistence Strategy

The frontend uses two persistence mechanisms:

| Mechanism | Storage Location | Purpose |
|-----------|-----------------|---------|
| **Workspace Layout** | `localStorage: dm-workspace-layout-{name}` | Run workspace panel positions and sizes |
| **Sidebar State** | `localStorage: dm-app-sidebar-open` / `dm-run-sidebar-open-{name}` | Global and run sidebar collapse state |
| **Language Preference** | `localStorage: dm-language` | i18n language selection |
| **Dark Mode** | Browser preference + `mode-watcher` | Theme switching |

Sources: [+layout.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+layout.svelte#L18-L29), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L58-L67), [i18n.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/i18n.ts#L10-L20)

---

After understanding the frontend skeleton and communication base, you can explore the following topics:

- **[Visual Graph Editor: SvelteFlow Canvas and YAML Synchronization](15-graph-editor)** — Deep dive into the `editor/+page.svelte` graph editor implementation, including node drag-and-drop, connection validation, and YAML bidirectional conversion
- **[Run Workspace: Grid Layout, Panel System, and Real-Time Interaction](16-runtime-workspace)** — GridStack integration details, panel lifecycle, and WebSocket message flow handling
- **[Frontend-Backend Integration: rust_embed Static Embedding and Release Process](23-build-and-embed)** — How production builds embed SvelteKit output into Rust binaries
