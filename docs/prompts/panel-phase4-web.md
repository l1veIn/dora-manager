# Phase 4: Web UI panel page

## Context

Read `docs/dm-panel-design.md` for architecture.
Phases 1-3 (dm-core, dm-cli, dm-server) are already implemented.

API endpoints available:
- `GET /api/panel/sessions` → `PanelSession[]`
- `GET /api/panel/:run_id/assets?since=<seq>&input_id=<id>&limit=<n>` → `PaginatedAssets`
- `GET /api/panel/:run_id/file/*path` → binary file
- `POST /api/panel/:run_id/commands` → `{ output_id, value }`

## Task

Build a **single Panel page** at `/panel`. No sub-routes.

### 1. API client

In `web/src/lib/api.ts`, add:

```typescript
export async function getPanelSessions(): Promise<PanelSession[]>
export async function getPanelAssets(runId: string, since?: number, inputId?: string): Promise<PaginatedAssets>
export function panelAssetUrl(runId: string, path: string): string  // construct URL
export async function sendPanelCommand(runId: string, outputId: string, value: string): Promise<void>
```

### 2. Page structure

Create `web/src/routes/panel/+page.svelte`:

```
┌───────────────────────────────────────────────────────┐
│  Panel                                                │
│  ┌──────────────┐  ┌──────────────────────────────┐   │
│  │ Sessions     │  │                              │   │
│  │              │  │  Selected session's           │   │
│  │ 🟢 abc123   │  │  asset widgets                │   │
│  │   my-robot   │  │                              │   │
│  │   17:00      │  │  [image] [text] [json] ...   │   │
│  │              │  │                              │   │
│  │ 📁 def456   │  │──────────────────────────────│   │
│  │   my-robot   │  │  Output Controls             │   │
│  │   10:15      │  │  speed: [____] [Send]        │   │
│  │              │  │                              │   │
│  │ 📁 ghi789   │  │──────────────────────────────│   │
│  │   test-flow  │  │  Timeline (replay mode)      │   │
│  │   yesterday  │  │  ◀ ■ ▶  ──●────── 1x        │   │
│  └──────────────┘  └──────────────────────────────┘   │
│                                                       │
│  No session selected? →  "Select a session to view"   │
└───────────────────────────────────────────────────────┘
```

#### URL parameter

`/panel?run=<run_id>` auto-selects a session. This is how the Runs page
links here — the 📊 button in each run row is `<a href="/panel?run={run.id}">`.

Use `$page.url.searchParams.get('run')` to read the parameter on mount.

#### Left sidebar: session list

- On mount, fetch `GET /api/panel/sessions`
- Display each session: run_id (truncated), dataflow name, timestamp, asset count
- Active sessions (recently modified) show 🟢 badge
- Clicking a session updates the selected `run_id` (reactive state) and
  adds `?run=<id>` to URL via `goto`

#### Right main area: asset view

Depends on selected session state:

| State | Behavior |
|-------|----------|
| No session selected | Empty state: "Select a session to view panel data" |
| Session selected, no assets | Empty state: "No panel data recorded for this run" |
| Session selected, active run | **Live mode**: poll every 100ms |
| Session selected, finished run | **Replay mode**: load all, show timeline |

### 3. Live mode

When selected session is active (detected by recent `last_modified`):

```javascript
let lastSeq = 0;
const interval = setInterval(async () => {
    const result = await getPanelAssets(runId, lastSeq);
    if (result.assets.length > 0) {
        assets = [...assets, ...result.assets];
        lastSeq = result.assets[result.assets.length - 1].seq;
    }
}, 100);
```

- Group assets by `input_id` — each input gets its own card/section
- For image inputs: only show the latest frame (replace, not accumulate)
- For text/JSON inputs: show as scrolling log (append, auto-scroll)

### 4. Replay mode

When selected session is finished:

- Load all assets (paginated, fetch in batches)
- Show timeline slider at bottom
- Dragging filters visible assets by timestamp range
- Play button auto-advances timestamp

### 5. Widget components

Create `web/src/lib/components/panel/`:

#### `AssetWidget.svelte` — dispatcher

```svelte
{#if asset.type.startsWith('image/')}
  <ImageWidget {asset} {runId} />
{:else if asset.type === 'text/plain'}
  <TextWidget {asset} />
{:else if asset.type === 'application/json'}
  <JsonWidget {asset} />
{:else}
  <RawWidget {asset} {runId} />
{/if}
```

#### `ImageWidget.svelte`

- `storage === 'file'`: `<img src={panelAssetUrl(runId, asset.path)} />`
- Show frame number and timestamp overlay
- Optional: fps counter

#### `TextWidget.svelte`

- Display `asset.data` (inline) in scrollable log view
- Auto-scroll in live mode

#### `JsonWidget.svelte`

- Pretty-print JSON from `asset.data`
- Collapsible tree view

#### `OutputControls.svelte`

- For each output_id: text input + Send button
- On submit: `sendPanelCommand(runId, outputId, value)`

#### `Timeline.svelte` (replay only)

- Range slider mapped to asset timestamp range
- Play/Pause/Speed controls

### 6. Runs page integration

In `web/src/routes/runs/+page.svelte`, add a 📊 icon button to each run row:

```svelte
<a href="/panel?run={run.id}" title="View panel data">📊</a>
```

Place it next to the existing 🗑️ delete button.

### 7. Navigation

In `web/src/routes/+layout.svelte`, add Panel to the sidebar nav:

```svelte
<a href="/panel">Panel</a>
```

### Design

Follow existing app patterns. Reference:
- `routes/runs/+page.svelte` for table and layout patterns
- `routes/events/+page.svelte` for polling and filter patterns
- Use existing CSS variables and component conventions

### Verification

```bash
cd /Users/yangchen/Desktop/dora-manager/web
npm run dev
# Open http://localhost:5173/panel
# Verify session list loads on left
# Verify clicking a session shows assets on right
# From Runs page, click 📊 → verify auto-selection works
# If a live dataflow with dm-panel is running, verify live polling
```
