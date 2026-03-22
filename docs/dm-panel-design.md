# DM Panel — Architecture Design

> Status: Design Crystallized (TEP S=3, Δ=5%)

## 1. Node Philosophy: Three-Layer Model

| Layer | Name | When to Use | Example |
|-------|------|-------------|---------|
| **Native** | Built into dm-core/dm-server | Management plane, works without a running dataflow | events, runs, dm-config |
| **Normal Node** | Opt-in dora node in YAML | Data plane, processes/produces dora messages | dm-panel |
| **Default Node** | Auto-injected by transpiler | Transparent to users, every dataflow gets it | (future: telemetry) |

The transpiler (`node: <id>` → `path: /abs/path`) gives dm the ability to inject
nodes between user YAML and dora execution YAML.

## 2. dm-panel: Three-in-One

Merges three originally separate proposals:

| Original | Direction | Merged Behavior |
|----------|-----------|----------------|
| dm-dashboard | DF → Web | inputs → store as assets → browser polls |
| dm-input | Web → DF | browser / CLI → INSERT command → dm-panel polls → inject |
| dm-recorder | DF → Disk | inherent — storage IS the first step |

### Why merge?

- No duplicate input declarations
- One Web UI page, one mental model: "add dm-panel to see your dataflow"
- Shared widget infrastructure between live view and replay
- Recording vs not-recording is just a **cleanup policy**
- TEP-validated: splitting costs more than three small responsibilities in one node

### Headless degradation

No `--headless` flag needed. Same YAML, same node:

| Component | Headless | With Browser |
|-----------|----------|-------------|
| inputs → storage | ✅ works | ✅ works |
| Browser polling | ⚪ nobody polls | ✅ real-time view |
| output (inject) | ⚪ no commands written | ✅ controls active |

## 3. "dm as Node" Pattern

dm-panel is compiled into the `dm` binary:

```
dm panel serve --run-id <id>
```

Transpiler generates:

```yaml
# user writes:
- id: panel
  node: dm-panel
  inputs:
    camera: yolo/bbox_image
  outputs:
    - speed

# transpiler produces for dora:
- id: panel
  path: dm
  args: ["panel", "serve", "--run-id", "{RUN_ID}"]
  inputs:
    camera: yolo/bbox_image
  outputs:
    - speed
```

Benefits: zero extra dependencies, no version sync, Rust performance,
direct `use dm_core::panel` access.

## 4. Unified Data Architecture: Everything is index.db

**Core principle: index.db is the sole communication channel.**

Both directions (dataflow ↔ outside world) go through
the same SQLite database. No networking in dm-panel at all.

```
Inbound:   dora input ──→ dm-panel ──→ INSERT assets  ──→ index.db ──→ dm-server polls ──→ browser
Outbound:  browser ──→ dm-server ──→ INSERT commands ──→ index.db ──→ dm-panel polls ──→ inject()
```

### Why no networking (no WebSocket, no TCP)?

- dm-panel becomes **pure filesystem I/O** — zero networking code
- No port management, no lifecycle cleanup, no connection failure handling
- Both directions automatically produce **audit logs** (assets table + commands table)
- SQLite WAL mode: safe for 1-writer-N-reader concurrency
- ~50ms poll latency is imperceptible for human-speed control

### Storage layout

```
~/.dm/
  ├── events.db                  ← existing: run metadata (separate scale)
  └── panel/
      └── <run_id>/              ← matches events.db case_id
          ├── index.db           ← assets + commands (WAL mode)
          ├── camera/
          │   ├── 000001.jpg
          │   └── ...
          └── lidar/
              └── 000001.pcd
```

### index.db schema

```sql
-- Inbound: dora → disk (dm-panel writes, dm-server reads)
CREATE TABLE assets (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    input_id  TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    type      TEXT NOT NULL,       -- "image/jpeg", "text/plain", etc.
    storage   TEXT NOT NULL,       -- "file" or "inline"
    path      TEXT,                -- relative path (binary data)
    data      TEXT                 -- inline content (text/json)
);

-- Outbound: outside → dora (dm-server/CLI writes, dm-panel reads)
CREATE TABLE commands (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    output_id TEXT NOT NULL,
    value     TEXT NOT NULL,
    timestamp TEXT NOT NULL
);
```

Both tables serve dual purpose: communication channel + audit log.
Replay can recreate both input data AND user control actions.

### Data type classification

Priority: dora metadata `content_type` > Arrow DataType > fallback (raw binary).

### Storage strategy

| Data Type | Storage | Rationale |
|-----------|---------|-----------|
| text / JSON / scalars | SQLite inline | queryable, small |
| images | filesystem (numbered frames) | large, HTTP-servable |
| audio / video | filesystem (container format) | sequential append |
| 3D point clouds | filesystem (.ply / .pcd) | specialized formats |

### Disk protection

`panel.max_storage` config (default: 2GB per run). Exceeded → rolling overwrite.

## 5. dm-panel Node Logic

**dm-panel has zero networking. Its only I/O: dora API + filesystem.**

```rust
fn panel_serve(home: &Path, run_id: &str) -> Result<()> {
    let store = PanelStore::open(home, run_id)?;  // index.db + WAL
    let (node, mut events) = DoraNode::init_from_node_id("dm-panel".into())?;
    let mut last_cmd_seq = 0;

    // Thread: handle dora inputs → write assets
    let store2 = store.clone();
    spawn(move || {
        while let Some(event) = events.recv() {
            if let Event::Input { id, metadata, data } = event {
                store2.write_asset(&id, &metadata, &data);
            }
        }
    });

    // Main: poll commands → inject outputs
    loop {
        for cmd in store.poll_commands(&mut last_cmd_seq)? {
            node.send_output(&cmd.output_id, to_arrow(&cmd.value)?)?;
        }
        sleep(Duration::from_millis(50));
    }
}
```

### Dependency direction

```
dm-panel:  reads/writes index.db only (zero outbound, zero networking)
dm-server: reads/writes index.db      (knows path convention)
dm CLI:    writes index.db             (knows path convention)
```

dm-panel has **zero knowledge** of dm-server. No `core → server` dependency.
Shared contract: path convention `~/.dm/panel/<run_id>/index.db`.

## 6. dm-config: Native Capability

NOT a dora node. Built into dm-core:

1. Reads all nodes' `dm.json` at dataflow load time
2. Aggregates config items into unified view
3. Web UI provides config editor per dataflow
4. No runtime hot-reload — dataflow restart is cheap

## 7. Web UI Integration

### Panel page (per-dataflow)

```
Dataflows → my-robot.yml → Panel
  ├── 🟢 Live — Run #abc123 (running)     ← poll mode
  ├── 📁 Run #def456 — 2026-03-05 10:15   ← replay mode
  └── 📁 Run #ghi789 — 2026-03-04 16:00   ← replay mode
```

- **Live**: polls `GET /api/panel/:run_id/assets?since=<seq>` at ~50-100ms
- **Replay**: loads asset index + commands, navigates by timeline, same widgets
- **Controls**: `POST /api/panel/:run_id/commands` → INSERT into commands table

Widget auto-detection by asset `type`. Extensible via custom widgets.

### Runs integration

- Panel sessions share `run_id` with existing runs module
- `dm runs clean` also cleans `panel/<run_id>/` directories

---

## 8. Crate-Level Changes

### dm-core

New `panel/` module — storage, query, models:

```
crates/dm-core/src/panel/
  ├── mod.rs          // pub API
  ├── model.rs        // Asset, AssetType, PanelSession, OutputCommand
  └── store.rs        // PanelStore: index.db + file writes
```

| API | Description |
|-----|-------------|
| `PanelStore::open(home, run_id)` | Create/open `panel/<run_id>/`, init `index.db` WAL |
| `PanelStore::write_asset(input_id, metadata, data)` | Classify + store + INSERT assets |
| `PanelStore::query_assets(filter)` | Query with pagination, `seq > N`, input_id filter |
| `PanelStore::write_command(output_id, value)` | INSERT commands (used by dm-server & CLI) |
| `PanelStore::poll_commands(&mut last_seq)` | SELECT commands WHERE seq > last (used by dm-panel) |
| `PanelStore::list_sessions(home)` | List sessions with stats |
| `PanelStore::clean(home, keep)` | Delete old sessions |

**No networking code.** Pure storage + query.

### dm-cli

```
crates/dm-cli/src/cmd/panel.rs
```

| Subcommand | Description |
|------------|-------------|
| `dm panel serve --run-id <id>` | **Internal**: dora node mode. `write_asset` + `poll_commands` loop |
| `dm panel send <output_id> <value>` | **User-facing**: calls `PanelStore::write_command()` directly |

`dm panel send` auto-discovers active sessions by scanning `~/.dm/panel/*/`
for recently-modified `index.db` files. No port files needed.

Transpiler: detect `node: dm-panel` → emit `path: dm, args: ["panel", "serve", ...]`

### dm-server

```
crates/dm-server/src/api/panel.rs
```

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/panel/sessions` | GET | List sessions |
| `/api/panel/:run_id/assets` | GET | Poll assets (`?since=<seq>&input_id=<id>`) |
| `/api/panel/:run_id/file/*path` | GET | Serve binary asset files |
| `/api/panel/:run_id/commands` | POST | Write command (`PanelStore::write_command`) |

**All HTTP. Zero WebSocket.**

### web (Svelte frontend)

```
web/src/routes/panel/
  ├── +page.svelte           // Session list
  └── [run_id]/
      └── +page.svelte       // Live/Replay view

web/src/lib/components/panel/
  ├── AssetWidget.svelte     // Auto-dispatch by type
  ├── ImageWidget.svelte
  ├── TextWidget.svelte
  ├── ChartWidget.svelte
  ├── OutputControls.svelte  // POST commands
  └── Timeline.svelte        // Replay with timeline
```

---

## 9. Architecture Diagram

```
┌──────── dora dataflow ──────────────────────────────────┐
│                                                         │
│  [yolo] ──image──→ [dm-panel (dm panel serve)]          │
│  [lidar] ──pcd──→       │                               │
│                    ┌────┴────┐                          │
│                    │ classify │    poll commands table   │
│                    │ + store  │ ←──── index.db ◄──────  │
│                    └────┬────┘          ↑               │
│                         │         write assets table    │
│                         ▼               │               │
│               ~/.dm/panel/<run_id>/     │               │
│               ├── index.db ─────────────┘               │
│               └── camera/*.jpg                          │
│                                                         │
└─────────────────────────────────────────────────────────┘
                          │
       dm-server          │          dm CLI
       read assets ◄──────┤────────► write commands
       write commands     │          (dm panel send)
       serve files        │
         ↑                │
      Browser             │
      (poll GET + POST)   │
```

---

## Appendix: TEP Validation

Validated via TEP v2.0 (3 rounds, 10 attacks, S=3, Δ=5%, CRYSTALIZED).
Post-TEP simplification: TCP eliminated in favor of index.db polling,
removing all networking from dm-panel. Full report: `tep_dm_panel.md`
