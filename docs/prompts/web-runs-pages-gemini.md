# DM Web Runs Pages Implementation Brief

This document is for implementing the web UI around the new DM run-instance
layer.

The target is to rebuild the old runs UI around the current backend model.

## Scope

Implement or refactor these pages:

- `/runs`
- `/runs/[id]`

Adjust these existing pages/components:

- dashboard page: make it reflect the current run-instance model
- dataflows page: connect it to run-instance actions and recent/current run info

Do not create a standalone top-level panel page as the main interaction model.
Panel should be embedded inside run detail when `has_panel == true`.

## Product model

The run instance is now the core business object.

A run owns:

- metadata (`run.json`)
- original dataflow snapshot
- transpiled dataflow snapshot
- logs
- panel assets/commands

Panel is not a separate session system anymore. It is a conditional sub-view of
a run.

## Pages

### 1. `/runs`

Purpose:

- primary index page for all run instances
- support browsing, filtering, pagination, and entering run detail

Data source:

- paginated runs API backed by `PaginatedRuns`

Expected fields per row:

- `id`
- `name`
- `status`
- `termination_reason`
- `outcome_summary`
- `started_at`
- `finished_at`
- `exit_code`
- `source`
- `has_panel`
- `node_count`
- `dora_uuid`

Recommended layout:

```text
Runs
┌ filters/search/pagination controls ────────────────────────────────┐
│ status filter | search | page size | prev/next                    │
└────────────────────────────────────────────────────────────────────┘

┌ run table / cards ────────────────────────────────────────────────┐
│ name | status | started | finished | nodes | panel | source      │
│ outcome summary                                                   │
│ click row -> /runs/[id]                                           │
└────────────────────────────────────────────────────────────────────┘
```

Required interactions:

- click row to open `/runs/[id]`
- filter by status: `running`, `succeeded`, `stopped`, `failed`, `all`
- search by `dataflow_name` and `run_id`
- pagination using `limit` + `offset`

Recommended defaults:

- page size: `20`
- sort: newest first

Empty state:

- show a simple empty state explaining no runs exist yet
- include link/button to go to the dataflows page

### 2. `/runs/[id]`

Purpose:

- operational workspace for one run
- status + logs + panel + snapshots in one place

This page is the most important new page.

Recommended layout:

```text
┌──────────────────────────────────────────────────────────────────────┐
│ Header                                                              │
│ dataflow_name             status badge                              │
│ run_id | started | duration | source | dora_uuid | has_panel        │
│ [Stop if running] [View dataflow.yml] [View transpiled.yml]         │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────┬───────────────────────────────┐
│ Left Column                          │ Right Column                  │
│                                      │                               │
│ Summary                              │ Node Logs                     │
│ Nodes                                │ large log viewer              │
│ Snapshots / metadata                 │ node selector / tail          │
│                                      │                               │
│                                      │ Panel (conditional)           │
│                                      │ timeline + asset feed         │
│                                      │ input box / send command      │
└──────────────────────────────────────┴───────────────────────────────┘
```

Design rule:

- left column = stable metadata
- right column = active interaction

If `has_panel == false`:

- do not render panel section
- let logs take the full right-side interaction area

#### Header section

Show:

- `dataflow_name`
- status badge
- `outcome_summary`
- `run_id`
- `dora_uuid`
- `started_at`
- `finished_at`
- derived duration
- `source`
- `has_panel`

Actions:

- `Stop` button only when run is still active
- open raw `dataflow.yml`
- open raw `dataflow.transpiled.yml`

If failed:

- show a prominent failure banner directly under header
- include:
  - `failure_node`
  - `failure_message`

#### Summary card

Show:

- `status`
- `termination_reason`
- `exit_code`
- `log_sync.state`
- `node_count_expected`
- `node_count_observed`

#### Nodes section

Show:

- expected/observed node inventory
- simple list of node ids
- highlight the selected node for logs

#### Logs section

Use the existing backend tail endpoint for live polling.

Required behavior:

- node selector
- fetch initial node log list
- live tail while run is active
- static read after run finishes
- preserve scroll usability for long logs

Recommended behavior:

- default selected node = first non-empty log if available
- show `(empty)` explicitly when a log exists but is empty

#### Panel section

Render only when `has_panel == true`.

Panel should behave like:

- timeline / feed of assets
- chat-like list for text/json updates
- file preview area for image/audio/video when possible
- command input box for sending panel commands

Do not model panel as a separate page-first concept.

Panel in detail view should support:

- polling assets
- rendering by asset type
- sending commands to the run panel

Useful asset rendering rules:

- `text/plain`: text block
- `application/json`: formatted JSON block
- `image/*`: image preview
- `audio/*`: audio player
- `video/*`: video player
- others: downloadable file item

## Dashboard page changes

Do not build a separate `/status` page.

Instead, update the current dashboard page so it reflects the run-instance
model.

Dashboard should show:

- runtime health
- active runs
- recent finished runs

Each active/recent run card should link to `/runs/[id]`.

Do not render raw Dora `list` table concepts as the main UI.
Use run metadata first.

Recommended dashboard sections:

- runtime health summary
- active runs
- recent finished runs

This page should feel like a lightweight operational overview, not a separate
analytics product.

## Dataflows page changes

Keep the dataflows page as the launch/edit page.

Add or update:

- `Run`
- `Force Run`
- `Stop current run` when applicable
- recent/latest run reference
- active run reference if the dataflow is currently running

After a successful run start:

- navigate to `/runs/[id]`

Useful UI additions:

- show current active run badge near the dataflow row/card
- show latest result badge from most recent run

The dataflows page should not become a run browser. It should remain a launch
surface.

## API expectations

Assume the backend already provides or can provide:

- paginated run list
- run detail
- run dataflow snapshot
- run transpiled snapshot
- stop run
- log tail
- panel asset query
- panel command send

Frontend should be written against run-first APIs, not old panel session APIs.

Use:

- `/api/runs/...`
- `/api/runs/{id}/panel/...`

Do not use old `/api/panel/sessions`.

## Suggested component split

Recommended reusable components:

- `RunStatusBadge`
- `RunSummaryCard`
- `RunTable`
- `RunHeader`
- `RunFailureBanner`
- `RunNodeList`
- `RunLogViewer`
- `RunPanelFeed`
- `RunPanelInput`
- `DataflowRunActions`
- `RecentRunCard`

Important reuse targets:

- status badge
- run summary card
- log viewer

## UX notes

- prefer fast scanning over dense tables everywhere
- logs and panel should be treated as first-class, not hidden under deep tabs
- use obvious failure styling for failed runs
- use explicit empty states
- keep pagination simple and predictable

## Known backend semantics to respect

- `has_panel` controls whether panel exists for a run
- panel is run-owned, not session-owned
- `status` can be `running`, `succeeded`, `stopped`, `failed`
- `termination_reason` is important and should be visible
- `failure_node` and `failure_message` may exist and should be surfaced clearly
- `outcome_summary` is already backend-prepared and should be reused

## Minimal implementation order

1. Rebuild `/runs`
2. Build `/runs/[id]`
3. Update dashboard with active/recent runs
4. Update dataflows run actions and post-start navigation

## Non-goals

- no separate primary panel page
- no Dora raw runtime table as the main runs UI
- no panel session concept in the frontend model
