# dm Web — UI Visual Design System

## Design Philosophy: "Mission Control"

dm is a **control center** for Dora-rs dataflows. Target users are robotics/AI developers
who live in terminals. The UI should feel like a natural visual extension of the CLI.

- **Information density over whitespace** — developers want data, not decoration
- **Status-driven visual hierarchy** — Dora runtime state is the most prominent element
- **Terminal DNA** — monospace fonts for code/YAML/logs, dark theme as default
- **Reference aesthetic** — Grafana · Portainer · Kubernetes Dashboard

## Tech Stack

| Layer | Choice |
|---|---|
| Framework | SvelteKit + Svelte 5 (Runes) |
| Components | shadcn-svelte (Bits UI) |
| Styling | Tailwind CSS v4 |
| Icons | Lucide Svelte |
| i18n | Paraglide (compile-time, zero runtime overhead) |
| Theme | mode-watcher (dark/light toggle, localStorage persistence) |
| Build | adapter-static (SPA), Vite proxy → `dm-server:3210` |

## Theme

- **Base color**: Slate (shadcn default)
- **Default mode**: Dark (developer tool convention)
- **Toggle**: Sun/Moon icon in sidebar footer, persisted via `mode-watcher`
- **Border radius**: `0.5rem`

### Typography
- **UI text**: `Inter` (Google Fonts) or system stack
- **Code/YAML/Logs**: `JetBrains Mono` (monospace)

### Status Colors
| State | Color | Badge |
|---|---|---|
| Running | `green-500` | `●  Running` |
| Stopped | `slate-400` | `○  Stopped` |
| Error | `red-500` | `✕  Error` |
| Installing | `amber-400` | `↻  Installing` |

## Layout

```
┌──────────────────────────────────────────────────────────┐
│  Header: Logo ("dm") · Dora Version Badge · Status Dot  │
├─────────────┬────────────────────────────────────────────┤
│  Sidebar    │  Main Content Area                         │
│  (240px)    │                                            │
│  Dashboard  │  (page content)                            │
│  Nodes      │                                            │
│  Editor     │                                            │
│  Events     │                                            │
│  Settings   │                                            │
│  ─────────  │                                            │
│  🌙/☀ Lang │                                            │
└─────────────┴────────────────────────────────────────────┘
```

- Sidebar uses shadcn `Sidebar` component, collapsible at `< lg`
- Footer: theme toggle + language switcher (en/zh-CN)

## i18n

- **Engine**: Paraglide (inlang) — compile-time translations
- **Languages**: `en` (default) + `zh-CN`
- **File structure**: `web/messages/en.json`, `web/messages/zh-CN.json`
- **Usage**: `import * as m from '$lib/paraglide/messages'` → `{m.some_key()}`

## API Client (`$lib/api.ts`)

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
├── +page.svelte          ← Dashboard (default)
├── nodes/+page.svelte    ← Node management
├── editor/+page.svelte   ← YAML editor + run
├── events/+page.svelte   ← Observability panel
└── settings/+page.svelte ← Config management
```
