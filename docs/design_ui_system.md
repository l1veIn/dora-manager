# dm Web — UI Visual Design System

## Tech Stack
- **Framework**: SvelteKit + Svelte 5 (Runes)
- **Components**: shadcn-svelte (Bits UI)
- **Styling**: Tailwind CSS v4
- **Icons**: Lucide Svelte
- **Build**: adapter-static (SPA mode), Vite proxy → `dm-server:3210`

## Design Tokens

### Theme
- **Base color**: Slate (shadcn default)
- **Mode**: Dark mode preferred (developer tool aesthetic), with light mode toggle
- **Border radius**: `0.5rem` (default shadcn)

### Typography
- **Font**: `Inter` via Google Fonts (or system `-apple-system, BlinkMacSystemFont`)
- **Heading scale**: Use Tailwind `text-2xl` → `text-sm`
- **Monospace**: `JetBrains Mono` for YAML editor and log output

### Color Semantics
| Token | Usage |
|---|---|
| `primary` | Action buttons, active states |
| `destructive` | Delete/uninstall actions |
| `muted` | Disabled, secondary text |
| `accent` | Hover highlights, badges |
| `chart-1..5` | Dora node status indicators |

### Status Colors
| State | Color | Badge |
|---|---|---|
| Running | `green-500` | `●  Running` |
| Stopped | `slate-400` | `○  Stopped` |
| Error | `red-500` | `✕  Error` |
| Installing | `amber-400` | `↻  Installing` |

## Layout Structure

```
┌──────────────────────────────────────────────────────────┐
│  Header: Logo ("dm") · Dora Version Badge · Status Dot  │
├─────────────┬────────────────────────────────────────────┤
│  Sidebar    │  Main Content Area                         │
│  (240px)    │                                            │
│             │  ┌──────────────────────────────────────┐  │
│  Dashboard  │  │  Page content rendered here          │  │
│  Nodes      │  │  (matches selected sidebar item)     │  │
│  Editor     │  │                                      │  │
│  Events     │  │                                      │  │
│  Settings   │  │                                      │  │
│             │  └──────────────────────────────────────┘  │
│             │                                            │
│  ─────────  │                                            │
│  Dark/Light │                                            │
└─────────────┴────────────────────────────────────────────┘
```

### Sidebar Component
Use `shadcn-svelte` **Sidebar** component:
- Collapsible (icon-only mode on narrow screens)
- Items: Dashboard, Nodes, Editor, Events, Settings
- Each item: Lucide icon + label
- Footer: theme toggle (Sun/Moon)

### Responsive Breakpoints
| Breakpoint | Behavior |
|---|---|
| `≥ lg (1024px)` | Sidebar expanded (240px) |
| `< lg` | Sidebar collapsed to icons or drawer mode |

## API Client Convention

All API calls go through a shared `$lib/api.ts` module:
```typescript
const API_BASE = '/api';

export async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
```

## Route Structure

```
web/src/routes/
├── +layout.svelte        ← Sidebar + Header shell
├── +layout.ts            ← ssr=false, prerender=false
├── +page.svelte          ← Dashboard (default page)
├── nodes/+page.svelte    ← Node management
├── editor/+page.svelte   ← YAML editor + run
├── events/+page.svelte   ← Observability panel
└── settings/+page.svelte ← Config management
```
