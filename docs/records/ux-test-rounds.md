# UX Test Rounds

## Round 1

### Goal

Test the quick-start flow and first-impression experience, with emphasis on obvious UI bugs and friction in getting Dora Manager running locally for the first time.

### Tester path

1. Follow the README startup path from the repo checkout.
2. Attempt to launch the app and open the web UI.
3. Inspect the dashboard as a first-time user.
4. Open a real dataflow from the UI and inspect the run page.

### Findings

- `dm-server` failed with `command not found` from the repo shell, even though the README presented it as the primary startup step.
- Dashboard frequent-dataflow shortcuts could point to non-existent workspaces such as `demo-hello-timer`, producing a broken first-click experience.
- Stopping a run could leave the page looking stuck in `Stopping...` while the backend was still processing the stop request.
- Dashboard history looked noisy and stale for a first-time user, without enough framing about what was local history versus current setup.

### Fixes shipped

- Updated startup guidance in `README.md` and `README_zh.md` to clearly separate installed-binary usage from source-checkout usage.
- Updated the dashboard frequent-dataflow cards to route to a valid destination:
  either the saved workspace, the latest run for that flow, or the dataflows list.
- Improved the dashboard empty-state and history framing so first-time users get clearer next steps and less misleading context.
- Updated the run page stop UX so stop requests remain visible as pending or delayed until the run reaches a terminal state or the request fails.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings.
- `cargo test -p dm-core stop_run_` passed after the backend stop fix.

### Re-pass results

- Repo-checkout startup guidance is now clear: the README separates installed-binary usage from source-checkout usage and points local development to `./dev.sh`.
- The stop flow no longer reproduced the old `Failed: dora stop timed out after 15s` outcome during the targeted re-pass. The run now finalized cleanly as `stopped` with `termination_reason: stopped_by_user`.
- There is still a short transition window where a stop request is acknowledged before the UI/backend fully converge on the terminal `stopped` state.
- The Dashboard frequent-dataflow shortcut remains under suspicion and should be the first item in the next UX pass, because tester observations were inconsistent with the new deployed card logic.

### Next focus

- Re-test the Dashboard frequent-dataflow cards end to end from the visible UI.
- Tighten the short `stopping` to `stopped` transition in the run detail if it still feels inconsistent in the real page flow.
- Continue the first-impression pass from Dashboard into Dataflows and the first successful run lifecycle.

## Round 2

### Goal

Create a deterministic first-success path from the Dashboard so the first meaningful click does not depend on stale history or missing saved workspaces.

### Tester path

1. Open the Dashboard.
2. Use the new canonical entry instead of relying on frequent history.
3. Launch a known-good run.
4. Inspect the run page.
5. Stop the run cleanly.

### Fixes shipped

- Added a new `Quick Start` section to [web/src/routes/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/+page.svelte).
- Added a built-in `Hello Timer Demo` Dashboard action that starts a known-good run directly through the app, without depending on saved workspaces or historical frequent-dataflow cards.
- Kept the existing Dashboard history and frequent-dataflow areas as secondary navigation rather than the only path to first success.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the Dashboard change.
- The rebuilt frontend contains the new `Quick Start`, `Hello Timer Demo`, and `Run Hello Timer` surfaces in the emitted app bundle.
- A manual `Computer Use` pass visually confirmed the canonical Dashboard path end to end:
  `Quick Start` was visible on the Dashboard, `Run Hello Timer` launched a real run, the run page showed `running` with `Active nodes 2 / 2`, and `Stop` finalized as `stopped` with `Stopped by user` and exit code `0`.

### Remaining issues

- The run summary still showed `Active Nodes 2 / 2` after the run had already reached terminal `stopped`, which is likely misleading state presentation.
- The message panel remained empty during the `Hello Timer` run; this may be expected for the current built-in flow, but it has not been verified yet.
- Frequent-dataflow behavior remains secondary until the post-stop run summary is truthful.

### Next focus

- Fix post-stop run detail truthfulness, especially summary cards that still imply live activity after a terminal stop.
- Confirm whether the empty message panel is expected for the built-in `Hello Timer` path or a real observability gap.
- If the run detail becomes trustworthy, move one step deeper into the edit -> rerun part of the main loop.

## Round 3

### Goal

Make the canonical `Hello Timer` run page trustworthy and self-explanatory during and after the first successful run.

### Tester path

1. Open the Dashboard.
2. Launch `Hello Timer Demo` from `Quick Start`.
3. Confirm the run page shows visible live output without any manual setup.
4. Stop the run.
5. Confirm the terminal run summary does not imply the run is still active.

### Findings

- The run summary labeled `node_count_observed` as `Active Nodes`, even though the backend field is a historical observed-node count derived from synced logs.
- The built-in `Hello Timer` path showed an empty `Message` panel because `dm-display` dropped null timer ticks instead of emitting visible text.

### Fixes shipped

- Updated [web/src/routes/runs/[id]/RunSummaryCard.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/[id]/RunSummaryCard.svelte) to relabel the run summary field as `Observed Nodes`, matching the actual backend semantics.
- Updated [nodes/dm-display/dm_display/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-display/dm_display/main.py) so null heartbeat events are rendered as visible `tick #N` text messages instead of being silently dropped.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings.
- API validation on `demo-hello-timer` confirmed `/api/runs/:id/messages` now returns `tick #N` text events from `dm-display`.
- A full `Computer Use` pass visually confirmed:
  - `Run Hello Timer` launches a live run from the Dashboard,
  - the `Message` panel fills with `tick #N` entries while running,
  - stop transitions through `stop requested`,
  - the terminal run page shows `Observed Nodes` instead of `Active Nodes`.

### Remaining issues

- Dashboard still exposes noisy historical sections and stale frequent-dataflow cards, which are now secondary but still affect first impression.
- The main validated path still stops at `run -> inspect -> stop`; the next product truth question is whether a first-time user can successfully cross into `edit -> rerun`.

### Next focus

- Decide whether the next round should:
  - keep reducing Dashboard history noise for first-time users, or
  - move deeper into the `edit -> rerun` loop and validate the first meaningful modification flow.

## Round 4

### Goal

Prove one deterministic `edit -> save -> rerun -> confirm change` loop starting from the Dashboard quick-start path.

### Tester path

1. Open the Dashboard.
2. Use the new `Edit Hello Timer` quick-start entry to open an editable workspace.
3. Make one small YAML change.
4. Save the workspace.
5. Rerun from the workspace page.
6. Confirm the behavior change from the run page itself.

### Findings

- The original quick-start flow launched a run, but did not provide a natural editable workspace path for the same demo.
- The run-page `Graph` modal could fail hard when a run had no persisted `view.json`, even when the YAML itself was available.

### Fixes shipped

- Updated [web/src/routes/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/+page.svelte) so `Hello Timer` quick start now exposes an `Edit Hello Timer` action that creates or opens a matching saved workspace.
- Updated the same Dashboard quick-start run path to attach a minimal `view_json` when launching the built-in demo.
- Updated [web/src/routes/runs/[id]/RunHeader.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/[id]/RunHeader.svelte) so the run-page `Graph` view falls back to an empty layout object instead of failing when `view.json` is missing.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the quick-start bridge changes.
- A full `Computer Use` pass visually confirmed:
  - Dashboard now shows `Edit Hello Timer`.
  - `Edit Hello Timer` opens `/dataflows/demo-hello-timer` as a real editable workspace.
  - Editing `dora/timer/millis/1000` to `dora/timer/millis/500`, saving, and clicking `Run` creates a new run with the updated YAML.
  - The rerun page visibly reflects the change: the runtime graph shows `Timer 500ms`, and the message panel advances at roughly 2 ticks per second (`tick #97` to `tick #121` within about one minute of runtime).

### Remaining issues

- Dashboard still surfaces noisy historical runs and stale frequent-dataflow cards that are no longer on the main path but still weaken first impression.
- Existing old runs launched before the quick-start view fix may still retain missing layout data, so historical artifacts remain uneven even though new runs are correct.

### Next focus

- Clean up first-impression noise on the Dashboard:
  - reduce stale frequent-dataflow fallbacks,
  - demote or contextualize old failed runs,
  - preserve the newly proven `run` and `edit -> rerun` paths as the obvious next actions.

## Round 5

### Goal

Make the Dashboard feel like a trustworthy starting surface instead of a mixed bag of old runs and fallback shortcuts.

### Tester path

1. Open the Dashboard after several prior local experiments already exist.
2. Check whether the quickest trustworthy actions are still obvious.
3. Inspect `Frequent Dataflows`.
4. Inspect `Recent Finished Runs`.
5. Judge whether stale or broken history still competes with the proven paths.

### Findings

- `Frequent Dataflows` still surfaced run-only fallback cards for workspaces that no longer existed, even though the dashboard already had real saved workspaces and a proven quick-start path.
- `Recent Finished Runs` only filtered `status === failed`, which let older `runtime_lost` history leak onto the homepage even though it reads as a failure to a new user.

### Fixes shipped

- Updated [web/src/routes/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/+page.svelte) so `Frequent Dataflows` now only shows saved workspaces that can actually be opened and edited from the Dashboard.
- Updated the same Dashboard view to treat `runtime_lost` and other failure-like outcomes as failure history for homepage filtering, instead of only checking literal `status === failed`.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the Dashboard filtering changes.
- A rebuilt local app and `Computer Use` pass confirmed:
  - `Quick Start` still presents `Run Hello Timer` and `Edit Hello Timer` as the clearest next actions.
  - `Frequent Dataflows` now only shows saved workspaces (`demo-hello-timer`, `stream`) instead of stale fallback run cards.
  - `Recent Finished Runs` no longer shows the older `runtime_lost` history entry that previously weakened trust on first view.

### Remaining issues

- `Recent Finished Runs` still includes older benign history, so the Dashboard is cleaner but not yet strongly time-bounded.
- Homepage trust is now better than breadth: the next question is whether the secondary history surfaces (`Runs`, `View All`) remain understandable once the homepage stops carrying so much context.

### Next focus

- Decide whether to:
  - tighten Dashboard recent-history bounds even further, or
  - move to the next major unknown outside the homepage, such as how understandable the full `Runs` history and recovery flow are for a first-time technical user.

## Round 6

### Goal

Prove one deterministic `failure -> diagnose -> fix -> rerun` loop from a real saved workspace, not just a happy-path first run.

### Tester path

1. Open `demo-hello-timer` as a saved workspace.
2. Introduce one small, controlled failure by renaming `dora-echo` to a missing node.
3. Save the workspace.
4. Observe how the workspace and run surfaces report the problem.
5. Restore the valid node name.
6. Confirm the workspace becomes runnable again without leaving the visible product path.
7. Rerun and confirm the product is healthy again.

### Findings

- A missing-node failure is a good first recovery test because it is low-risk, easy to restore, and directly relevant to real node-based workflows.
- Before the fix in this round, saved YAML changes could leave the workspace page visually stale until a full page refresh, even though the backend had already recomputed the workspace as `missing_nodes` or `ready`.
- That stale state was dangerous in recovery mode: the API already knew the truth, but the page could still show the previous badge and action state.

### Fixes shipped

- Updated [web/src/routes/dataflows/[id]/components/YamlEditorTab.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/dataflows/[id]/components/YamlEditorTab.svelte) so a successful YAML save now fetches the refreshed workspace payload from the backend and returns it to the parent page.
- Updated [web/src/routes/dataflows/[id]/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/dataflows/[id]/+page.svelte) so the workspace page immediately swaps in the refreshed backend truth after save, instead of waiting for a manual reload.
- Added a persistent `lastRunError` banner surface on the same page so future run-start failures are not limited to a disappearing toast.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the workspace recovery changes.
- `Computer Use` verified the recovery loop end to end:
  - changing `dora-echo` to `dora-echo-missing` and saving now flips the workspace to `Missing Nodes` immediately,
  - the `Run` button becomes disabled without a full page refresh,
  - restoring `dora-echo` and saving flips the workspace back to `Ready` immediately,
  - the restored workspace can be run again successfully,
  - the rerun reaches a healthy live run page with ticking output and stops cleanly as `Stopped by user`.
- Backend API checks confirmed the same workspace truth used in the UI:
  - broken state: `status=missing_nodes`, `can_run=false`
  - restored state: `status=ready`, `can_run=true`

### Remaining issues

- `Missing Nodes` is now truthful and immediate, but still not fully self-explanatory; the page does not yet clearly summarize which nodes are missing or suggest the next corrective action.
- The new persistent `lastRunError` banner shipped in this round, but this pass primarily validated the stronger save-time executable sync; a runtime failure that bypasses preflight still deserves a dedicated follow-up check.

### Next focus

- Make broken workspaces explain themselves before the user clicks `Run`:
  - show which nodes are missing,
  - explain what action to take next,
  - keep the recovery path inside the workspace page instead of relying on guesswork or console output.

## Round 7

### Goal

Make a broken workspace self-explanatory before the user even attempts another run.

### Tester path

1. Put `demo-hello-timer` into a controlled `missing_nodes` state.
2. Open the workspace page directly.
3. Judge whether the page itself explains:
   - what is wrong,
   - which node is missing,
   - what the user should do next,
   - whether the workspace is safely prevented from running.
4. Restore the workspace and confirm the warning disappears with the page returning to `Ready`.

### Findings

- `Missing Nodes` as a badge is truthful, but by itself it is too terse for a first-time technical user.
- The workspace page already had all the backend data needed to be more explicit: missing node names, optional import sources, and a known non-runnable state.
- Once the save-time executable sync landed in Round 6, the main remaining gap was presentation, not backend truth.

### Fixes shipped

- Updated [web/src/routes/dataflows/[id]/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/dataflows/[id]/+page.svelte) to render explicit workspace issue banners for:
  - `invalid_yaml`
  - `missing_nodes`
- The missing-node banner now states the number of missing nodes, lists them explicitly, and tells the user to restore or install them before saving and rerunning.
- If the backend knows a Git source for a missing node, the page now renders a concrete `dm node import ...` hint inline.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the workspace issue banner changes.
- `Computer Use` validated the broken-workspace path visually:
  - the workspace shows a red `Missing Nodes` badge,
  - `Run` is disabled,
  - the page now includes an in-place warning banner that says the workspace is missing `1` node,
  - the banner lists `dora-echo-missing` explicitly and tells the user to install or restore the node before saving again.
- After restoring the workspace to a valid YAML, a reload confirmed the warning banner disappears and the page returns to `Ready`.

### Remaining issues

- This round validated the `missing_nodes` explanation path, but not the `invalid_yaml` banner with a real syntax error sample.
- Runtime failures that still pass executable preflight remain a separate class; they rely on the new `lastRunError` surface from Round 6 and need their own focused pass.

### Next focus

- Extend recovery clarity beyond missing nodes:
  - validate and improve the `invalid_yaml` path,
  - test one failure that only appears after clicking `Run`,
  - ensure those failures remain visible long enough for a first-time user to act on them.

## Round 8

### Goal

Validate the two remaining recovery surfaces identified in Cycle 6:

1. an `invalid_yaml` workspace before run
2. a run-start failure that only appears after clicking `Run`

### Tester path

1. Put `demo-hello-timer` into a controlled `invalid_yaml` state.
2. Confirm the backend marks it non-runnable and exposes a parse error.
3. Restore the workspace.
4. Put the same workspace into a controlled external-path configuration that passes executable preflight but points to a missing binary.
5. Click `Run`.
6. Confirm the failure remains visible on the workspace page itself after the toast disappears.
7. Restore the workspace again.

### Findings

- `invalid_yaml` is a distinct broken-workspace class and already had enough backend data to support a useful page-level banner.
- A missing external `path:` is a good test for post-click failures because the workspace still appears `Ready`, but run start fails in a realistic way.
- The new `lastRunError` surface from Round 6 is necessary: without it, this second class would collapse back into a disappearing toast plus a vague return to the workspace page.

### Fixes shipped

- No additional code changes were required in this round beyond the recovery surfaces added in Rounds 6 and 7.
- The work in this round was focused on validating that those surfaces actually cover both pre-run and post-click failure classes.

### Validation

- API validation confirmed the `invalid_yaml` path produces:
  - `status=invalid_yaml`
  - `can_run=false`
  - a concrete parse error message in `executable.error`
- A headless browser pass validated the post-click failure path for a missing external binary:
  - before clicking `Run`, the workspace still showed `Ready`,
  - after clicking `Run`, the page displayed both the transient toast and the persistent `Last run attempt failed` banner,
  - the persistent banner retained the failure cause on the page after the toast, including the missing path and spawn failure context.
- The test workspace was restored to a clean `ready` state after validation.

### Remaining issues

- The persistent run-failure banner is useful, but the raw failure text is still too verbose and backend-shaped for first-time users.
- The `invalid_yaml` page path is now backed by the right data and rendering logic, but this round relied on backend/API confirmation rather than a separate full visual pass because local GUI tooling became unreliable mid-run.

### Next focus

- Compress and normalize post-click failure messaging:
  - keep the root cause,
  - remove stack-trace noise,
  - preserve enough detail for technical users without making the workspace page read like backend stderr.

## Round 9

### Goal

Make post-click run failures readable enough that a first-time technical user can recover from the workspace page without first decoding raw backend stderr.

### Tester path

1. Put `demo-hello-timer` into a controlled external-path configuration that still passes executable preflight.
2. Open the workspace page while it still appears `Ready`.
3. Click `Run`.
4. Judge whether the toast and persistent workspace banner:
   - keep the real cause,
   - remove obvious backend noise,
   - explain what to do next,
   - still preserve raw technical detail somewhere on the page.
5. Restore the workspace after validation.

### Findings

- The failure was truthful but too raw: the primary surface showed trigger IDs, `[ERROR]`, `Caused by`, and multiple `Location:` blocks copied directly from the backend response.
- That shape made the product feel like an internal tool even though the recovery loop itself was already working.
- The right fix was not new backend semantics; it was a cleaner error-shaping layer on the workspace page plus better API error extraction.

### Fixes shipped

- Updated [web/src/lib/api.ts](/Users/yangchen/Desktop/dora-manager/web/src/lib/api.ts) so non-2xx responses now extract the most useful message from JSON error bodies instead of blindly surfacing raw response text.
- Updated [web/src/routes/dataflows/[id]/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/dataflows/[id]/+page.svelte) so run-start failures are normalized into:
  - a concise summary,
  - short visible detail lines,
  - the existing recovery hint,
  - an expandable raw technical detail block for full backend output.
- The workspace banner now strips trigger IDs, `[ERROR]`, and repeated `Location:` frames from the main reading path while preserving the underlying cause.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the error-shaping changes.
- A controlled headless browser pass on `/dataflows/demo-hello-timer` validated the post-click failure path:
  - the workspace still appeared `Ready` before clicking `Run`,
  - after clicking `Run`, the banner summary read `Failed to spawn echo`,
  - the primary banner no longer showed `dataflow start triggered:` or any `Location:` blocks,
  - the banner kept the actionable root cause (`failed to run '/tmp/definitely-missing-dm-node'` and `No such file or directory (os error 2)`),
  - the page still preserved a `Show raw technical detail` toggle for the full backend payload.
- The workspace was restored to a clean `ready` state after the pass.

### Remaining issues

- The workspace failure surface is now productized, but failed runs recorded in history still keep the raw `outcome_summary` string from the backend.
- As a result, the same failure looks cleaner on the workspace page than it does on `Runs` history and run detail surfaces.

### Next focus

- Normalize failed-run history surfaces so `Runs` and run detail pages do not reintroduce the same raw backend noise that was just removed from the workspace page.

## Round 10

### Goal

Align failed-run history surfaces with the cleaned workspace failure messaging so the product does not switch back to raw backend wording once the user leaves the workspace page.

### Tester path

1. Reuse an existing `start_failed` run created during the controlled missing-binary tests.
2. Open the homepage and inspect recent failed-run cards.
3. Open the `Runs` page and inspect the failed run row.
4. Open the failed run detail page.
5. Judge whether all three surfaces present the same concise failure summary without leaking raw trigger IDs or `Location:` traces.

### Findings

- The run data itself was already adequate, but the display layer still rendered raw multi-line `outcome_summary` text directly on history surfaces.
- That meant the product told a cleaner story on the workspace page than it did on the homepage, `Runs`, or run detail header.
- This was a presentation consistency problem, not a backend-state problem.

### Fixes shipped

- Added [web/src/lib/runs/outcomeSummary.ts](/Users/yangchen/Desktop/dora-manager/web/src/lib/runs/outcomeSummary.ts) to normalize multiline failed-run summaries into concise history-safe text while preserving single-line summaries unchanged.
- Updated [web/src/lib/components/runs/RecentRunCard.svelte](/Users/yangchen/Desktop/dora-manager/web/src/lib/components/runs/RecentRunCard.svelte), [web/src/routes/runs/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/+page.svelte), and [web/src/routes/runs/[id]/RunHeader.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/[id]/RunHeader.svelte) to use the normalized display summary instead of the raw stored `outcome_summary`.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the summary-normalization change.
- A headless browser pass validated:
  - the `Runs` page no longer displayed `dataflow start triggered:` or any `Location:` blocks,
  - the failed run row instead showed `Failed to spawn echo - No such file or directory (os error 2)`,
  - the run detail header showed the same normalized summary,
  - the homepage no longer leaked raw trace content into recent run cards.

### Remaining issues

- The current failure normalization is still heuristic and display-only; new failure classes may need additional summary shaping if they produce very different backend text.
- The next major uncertainty is no longer recovery wording. It is which broader browsing or onboarding surface now carries the highest remaining first-time friction.

### Next focus

- Re-evaluate the product from a first-time technical user angle and choose the next high-friction surface outside the now-hardened quick-start and recovery flows.

## Round 11

### Goal

Make `Dataflows` understandable as the next-step exploration surface after first success, instead of leaving users to infer the product model from a flat list of saved workspaces.

### Tester path

1. Open `Dataflows` after completing the proven quick-start path.
2. Judge whether the page explains:
   - what `Run` does,
   - what `Workspace` does,
   - which workspaces are recommended next,
   - which items are broken or need extra setup.
3. Verify that recommended and attention-needed items are visually separated.

### Findings

- The original `Dataflows` page behaved like a raw inventory list.
- It did not explain whether `Run` or `Workspace` was the right next action, and it mixed runnable workspaces with broken artifacts in one continuous feed.
- That was a product-orientation problem, not a backend-truth problem.

### Fixes shipped

- Updated [web/src/routes/dataflows/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/dataflows/+page.svelte) to add a top-level orientation panel describing:
  - that the page is the saved workspace map,
  - what `Run` means,
  - what `Workspace` means,
  - which workspaces are the best next exploration targets after the first demo.
- Added explicit status badges for all listed workspaces, including `Ready`.
- Split the page into:
  - `Ready To Explore`
  - `Needs Attention Or Extra Setup`
- Marked `demo-hello-timer` and `interaction-demo` as recommended next steps and pushed broken or setup-dependent items into the lower section with repair hints.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the `Dataflows` page change.
- A headless browser pass validated:
  - the page now includes the saved-workspace map explanation,
  - `Run` and `Workspace` semantics are spelled out in-page,
  - recommended next workspaces are visibly marked,
  - broken items are grouped below with explicit repair cues such as `Restore dm-panel`.

### Remaining issues

- This round improved orientation on the `Dataflows` page itself, but it has not yet proven the full “second task” journey from a recommended non-hello-timer workspace into edit, run, and result confirmation.
- The broader cross-surface mental model is stronger, but it still needs one realistic follow-up pass outside the original `Hello Timer` loop.

### Next focus

- Validate one real “second task” journey starting from a recommended workspace beyond the original quick-start flow, most likely `interaction-demo`.

## Round 12

### Goal

Respond to direct human feedback about the `Nodes` surface by making the node catalog understandable, filterable, and actionable for both discovery and custom-node authoring.

### Tester path

1. Review the `Nodes` page as a human user trying to distinguish builtin nodes, custom nodes, imported nodes, and not-yet-installed entries.
2. Try to find a specific node quickly.
3. Create a new node and observe where it lands.
4. Open the new node and check whether the page offers a clear “continue editing” path.
5. Validate that node avatars declared in `display.avatar` are actually renderable.

### Findings

- The original page mislabeled itself as `Installed Nodes` even though `/api/nodes` returns a mixed catalog of builtin, local, imported, installed, and not-yet-installed nodes.
- Node cards were wasting available metadata: source, category, maintainers, runtime language, and avatar support were effectively hidden.
- New nodes were easy to lose because they were not promoted in sorting and the create flow reset the ID before redirecting, sending the user to `/nodes?new=1` instead of the node detail page.
- The backend and wiki already supported `display.avatar`, but the UI had no way to render it because node assets were only exposed as text files.

### Fixes shipped

- Added [web/src/lib/nodes/catalog.ts](/Users/yangchen/Desktop/dora-manager/web/src/lib/nodes/catalog.ts) to centralize node-origin, runtime, maintainer, sorting, and avatar helpers.
- Reworked [web/src/routes/nodes/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/nodes/+page.svelte) into a real catalog surface:
  - renamed the main section to `Node Catalog`,
  - added status and origin filters,
  - added paging,
  - added summary counts,
  - sorted local/custom work ahead of builtin noise,
  - clarified that the page is a mixed catalog rather than an installed-only list.
- Upgraded [web/src/routes/nodes/NodeCard.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/nodes/NodeCard.svelte) to show origin, category, install state, runtime, maintainer, tags, and avatar when present.
- Fixed [web/src/routes/nodes/CreateNodeDialog.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/nodes/CreateNodeDialog.svelte) so newly created node IDs are preserved through the callback and redirect.
- Upgraded [web/src/routes/nodes/[id]/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/nodes/[id]/+page.svelte) to show origin/category metadata, a `Node scaffold created` notice, and an `Open With` menu for VS Code, Finder, and Terminal.
- Added backend support for opening node directories and serving node asset files:
  - [crates/dm-core/src/node/local.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/node/local.rs)
  - [crates/dm-core/src/node/mod.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/node/mod.rs)
  - [crates/dm-server/src/handlers/nodes.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/nodes.rs)
  - [crates/dm-server/src/handlers/mod.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/mod.rs)
  - [crates/dm-server/src/main.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-server/src/main.rs)

### Validation

- `cargo check -p dm-server` passed.
- `npm run check` in `web/` passed with 0 errors and 0 warnings.
- A headless browser pass validated that the `Nodes` page now includes:
  - catalog framing copy,
  - install-state filters,
  - origin filters,
  - the new count summary.
- A real create-node pass validated:
  - new node creation now lands on `/nodes/<id>?new=1`,
  - the detail page shows the `Node scaffold created` notice,
  - the `Open With` control is present,
  - the page clearly reads as a local editable node workspace.
- A temporary avatar validation pass confirmed that when a node’s `dm.json` sets `display.avatar`, both the catalog card and the node detail page request and render the image through `/api/nodes/<id>/artifacts/...`.

### Remaining issues

- Node metadata still does not have a separate `cover` field in the current model; only `display.avatar` is wired end to end.
- Origin classification is currently inferred at the UI layer from existing metadata and paths; if stronger guarantees are needed, the backend should eventually expose an explicit node-origin field.
- The interaction-demo “second task” pass was interrupted by this inserted node-catalog repair round and still deserves a dedicated follow-up.

### Next focus

- Return to the deferred “second task” flow, or continue deepening node-catalog quality by pushing explicit node-origin semantics into the backend API.

## Round 13

### Goal

Resume the interrupted `interaction-demo` second-task flow and make the interactive run path self-explanatory for first-time technical users.

### Tester path

1. Open `interaction-demo` from the now-improved `Dataflows` surface.
2. Run it as a real user without consulting docs.
3. Judge whether it is obvious:
   - where to type input,
   - how to submit it,
   - where to look for the response.
4. Verify whether the interaction protocol itself works by comparing page behavior with direct API behavior.

### Findings

- The runtime interaction protocol was healthy: direct `input` messages posted to the run produced echoed text output immediately.
- The real product weakness was affordance and guidance:
  - the run page did not explicitly explain the `Input` -> `Message` loop,
  - the textarea submit action relied on a tiny icon and `Cmd/Ctrl+Enter`, which is too hidden for a first-time user.
- This was a UX/explanation gap, not a backend protocol failure.

### Fixes shipped

- Updated [web/src/lib/components/workspace/panels/input/controls/ControlTextarea.svelte](/Users/yangchen/Desktop/dora-manager/web/src/lib/components/workspace/panels/input/controls/ControlTextarea.svelte) so textarea widgets now show:
  - a visible `Send` button,
  - a visible shortcut hint,
  - a less hidden submission path than the old icon-only affordance.
- Updated [web/src/lib/components/workspace/panels/input/controls/ControlInput.svelte](/Users/yangchen/Desktop/dora-manager/web/src/lib/components/workspace/panels/input/controls/ControlInput.svelte) to use the same clearer send affordance pattern.
- Updated [web/src/routes/runs/[id]/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/[id]/+page.svelte) to show an interaction guidance banner whenever the run exposes widget inputs, explicitly telling the user to submit input in the `Input` panel and watch the result in `Message`.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings after the interaction-surface changes.
- Direct API validation on a live `interaction-demo` run confirmed:
  - posting `tag=input` to `/api/runs/:id/messages` updated `interaction.current_values`,
  - `display` emitted a `text` stream containing the echoed value.
- A headless browser pass validated that the run page now visibly includes:
  - the `This run is interactive` guidance banner,
  - explicit references to `Input` and `Message`,
  - a visible `Send` control and shortcut hint in the input widget itself.

### Remaining issues

- This round made the interaction path legible, but it did not complete a full automated end-to-end browser proof that the visible `Send` button dispatches input successfully through the UI.
- The next useful pass should confirm that visible interaction submission now works end to end from the browser, not only through API injection.

### Next focus

- Complete one end-to-end browser-verified interactive submission loop for `interaction-demo`, or move to the next highest-friction product surface if that loop is already manually trustworthy.

## Round 14

### Goal

Respond to direct human feedback on run-stop trust by making the interactive hint dismissible and making `Stop` behave like a durable lifecycle state instead of a page-local illusion.

### Tester path

1. Open a live `interaction-demo` run.
2. Confirm the interaction hint appears, then dismiss it and refresh.
3. Click `Stop`.
4. Refresh or revisit the run page while the runtime is still draining.
5. Confirm the product still shows a truthful `stopping` state and explains that it is safe to leave.
6. Wait for the runtime to settle and confirm the terminal state replaces the temporary stop-request state.

### Findings

- The interaction guidance copy was now useful, but it had become a permanent banner with no exit path.
- The run page’s `stop requested` state was only local frontend memory. If the user refreshed or navigated away and came back, the product fell back to plain `running` even though the stop had already been accepted.
- The backend did not persist any stop-request fact, so the product had no way to distinguish “normally running” from “stop has been requested and is still draining.”
- The current stop path still relies on a synchronous `dora stop` call. In live validation, an `interaction-demo` stop took about 12 seconds to settle, which is long enough that pretending the run is still ordinary `running` materially damages trust.

### Fixes shipped

- Added persistent stop-request metadata to the run model in:
  - [crates/dm-core/src/runs/model.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/model.rs)
  - [crates/dm-core/src/runs/service_query.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/service_query.rs)
  - [crates/dm-core/src/runs/state.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/state.rs)
- Added a dedicated `mark_stop_requested` runtime step and persisted timeout diagnostics in:
  - [crates/dm-core/src/runs/service_runtime.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/service_runtime.rs)
  - [crates/dm-core/src/runs/service.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/service.rs)
  - [crates/dm-core/src/runs/mod.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/mod.rs)
- Updated [crates/dm-core/src/runs/service_runtime.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/service_runtime.rs) refresh semantics so a persisted stop request survives runtime reconciliation:
  if Dora later reports `Succeeded`, `Stopped`, or disappearance after a stop request, dm now resolves the run as `Stopped by user` instead of misclassifying it as a normal completion or generic runtime loss.
- Updated [crates/dm-server/src/handlers/runs.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/runs.rs) so `/api/runs/:id/stop` persists the stop request before spawning the background stop task and returns `stop_requested_at` plus `can_leave: true`.
- Tightened the synchronous stop wait budget in [crates/dm-core/src/runs/runtime.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/runtime.rs) from `45s` to `15s`. This does not guarantee the runtime finishes in 15 seconds; it limits how long `dm` waits before leaving the run in a truthful background-draining state.
- Updated the run UI in:
  - [web/src/routes/runs/[id]/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/[id]/+page.svelte)
  - [web/src/routes/runs/[id]/RunHeader.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/[id]/RunHeader.svelte)
  - [web/src/lib/components/runs/RunStatusBadge.svelte](/Users/yangchen/Desktop/dora-manager/web/src/lib/components/runs/RunStatusBadge.svelte)
  - [web/src/lib/components/runs/RecentRunCard.svelte](/Users/yangchen/Desktop/dora-manager/web/src/lib/components/runs/RecentRunCard.svelte)
  - [web/src/routes/runs/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/runs/+page.svelte)
- The interaction guidance banner is now dismissible and persisted in local storage per run/dataflow name, so it stays gone after reload once the user has acknowledged it.

### Validation

- `npm run check` in `web/` passed with 0 errors and 0 warnings.
- `cargo test -p dm-core stop_run_` passed.
- `cargo test -p dm-core mark_stop_requested_persists_background_stop_state` passed.
- `cargo test -p dm-core refresh_run_statuses_preserves_user_stop_intent_across_runtime_terminal_states` passed.
- `cargo check -p dm-server` passed.
- A real browser-driven CDP pass validated that:
  - `interaction-demo` now shows the interactive hint, a dismiss button, and an explicit `Send` control,
  - dismissing the hint removes it immediately and it stays dismissed after reload,
  - clicking `Stop` changes the run page to a visible `stopping` state,
  - the page explains it is safe to leave or refresh while the stop drains,
  - refreshing the page during drain preserves the `stopping` state instead of reverting to plain `running`,
  - `/api/runs/:id` exposes `stop_requested_at` while the runtime is still draining and clears it once the run reaches terminal `stopped`.

### Remaining issues

- This round made stop state truthful, but it did not make the underlying runtime shutdown itself fast. Direct measurement showed `dora stop <uuid>` itself taking about 11.75 seconds on `interaction-demo`, and `dora list` continued to report the flow as `Running` until it eventually flipped terminal.
- The next lifecycle round should investigate whether some classes of nodes or flows are slow to acknowledge Dora stop, rather than making the UI guess differently.
- The interactive submission path is now more legible and dismissible, but a future round should still keep deepening “actual input sent -> response observed” as a first-class browser proof.

### Next focus

- Investigate why real stop latency remains high for interactive flows, now that the product surfaces are no longer lying about the state.

## Round 15

### Goal

Investigate the real source of stop latency and determine whether the slowdown belongs to Dora runtime itself or to dm-owned interactive widget nodes.

### Tester path

1. Measure direct stop time on `interaction-demo`.
2. Compare it with a non-interactive flow such as `demo-hello-timer`.
3. Observe which processes remain alive during the slow stop window.
4. Run a causality test by terminating the suspected lingering process during stop.
5. If the blocker is dm-owned, apply the smallest node-layer fix and re-measure the dm user path.

### Findings

- The stop slowdown was not universal:
  - direct `dora stop` on `interaction-demo` took about `11.75s`,
  - direct `dora stop` on `demo-hello-timer` took about `0.04s`.
- Process observation showed that during slow stop:
  - `dm-display` and `dora-echo` exited almost immediately,
  - `dm-text-input` remained alive for the full delay window,
  - `dora list` kept reporting the flow as `Running` until `dm-text-input` finally disappeared.
- A causality test confirmed the blocker:
  manually sending `SIGTERM` to the lingering `dm-text-input` process at about `2s` caused `dora stop` to return around `3s`.
- The root cause was not generic Web UI latency. It was a dm widget-node pattern:
  `dm-text-input`, `dm-button`, `dm-slider`, and `dm-input-switch` all run their own websocket reconnect loop against dm-server, and they previously had no way to notice that dm had already accepted a stop request unless Dora delivered a timely signal.

### Fixes shipped

- Updated the interactive widget nodes to become stop-aware by polling run state from dm-server and exiting their websocket loop once the run is no longer truly active:
  - [nodes/dm-text-input/dm_text_input/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-text-input/dm_text_input/main.py)
  - [nodes/dm-button/dm_button/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-button/dm_button/main.py)
  - [nodes/dm-slider/dm_slider/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-slider/dm_slider/main.py)
  - [nodes/dm-input-switch/dm_input_switch/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-input-switch/dm_input_switch/main.py)
- The new logic makes these nodes exit when `/api/runs/:id` reports either:
  - `stop_requested_at` is present, or
  - the run is no longer `running`.

### Validation

- `python3 -m py_compile` passed for all four updated widget-node entrypoints.
- `npm run check` in `web/` still passed with 0 errors and 0 warnings.
- A real dm-driven stop measurement on `interaction-demo` after the widget fix showed:
  - `POST /api/runs/:id/stop` to terminal `stopped_by_user` now completed in about `2190ms`.
- This confirmed that the user-facing dm stop path improved from roughly `11-12s` down to about `2.2s` for the investigated interactive flow.

### Remaining issues

- Direct raw `dora stop` still takes the old slow path if it bypasses dm's persisted stop-request semantics, because the widget fix intentionally keys off dm run state.
- This round strongly improves the dm product path, but it does not solve shutdown latency for users who invoke Dora directly outside dm.
- Other dm nodes with long-lived side channels should still be audited for the same “reconnect forever until signaled” pattern.

### Next focus

- Decide whether to leave raw Dora stop behavior as-is and optimize only the dm-managed product path, or to pursue a deeper runtime/node contract that also improves direct `dora stop` semantics outside dm.

## Round 16

### Goal

Turn the newly agreed DM-plane ontology into an explicit capability-binding contract and validate it on a minimal set of builtin interaction nodes before merge.

### Tester path

1. Rebuild context from the panel ontology discussion and current node metadata.
2. Formalize the `dm` capability-binding contract in a design document.
3. Migrate the smallest realistic builtin pilot set:
   - `dm-text-input`
   - `dm-display`
   - companion widget nodes in the same family
4. Verify that node APIs now expose typed DM bindings.
5. Verify that node-facing UI surfaces acknowledge the DM plane instead of hiding it completely.

### Findings

- The repo already contained an implicit DM plane:
  ad hoc `interaction` metadata in `dm.json`, run-scoped message persistence, widget registration, and browser-facing input/display flows.
- That DM plane was real but structurally hidden. Nodes could participate in it, but the node model did not type it and the product did not surface it clearly.
- The immediate goal was therefore not to redesign transport, but to stop treating DM-plane semantics as untyped leftovers.

### Fixes shipped

- Added the new design document:
  [docs/design/dm-capability-binding-v0.md](/Users/yangchen/Desktop/dora-manager/docs/design/dm-capability-binding-v0.md)
- Extended the node model with first-class DM-plane metadata:
  - [crates/dm-core/src/node/model.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/node/model.rs)
  - [crates/dm-core/src/node/mod.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/node/mod.rs)
- Migrated the builtin interaction nodes to the new `dm.version = \"v0\"` binding contract:
  - [nodes/dm-text-input/dm.json](/Users/yangchen/Desktop/dora-manager/nodes/dm-text-input/dm.json)
  - [nodes/dm-display/dm.json](/Users/yangchen/Desktop/dora-manager/nodes/dm-display/dm.json)
  - [nodes/dm-button/dm.json](/Users/yangchen/Desktop/dora-manager/nodes/dm-button/dm.json)
  - [nodes/dm-slider/dm.json](/Users/yangchen/Desktop/dora-manager/nodes/dm-slider/dm.json)
  - [nodes/dm-input-switch/dm.json](/Users/yangchen/Desktop/dora-manager/nodes/dm-input-switch/dm.json)
- Updated the node detail page to show the DM plane explicitly instead of keeping it implicit:
  [web/src/routes/nodes/[id]/+page.svelte](/Users/yangchen/Desktop/dora-manager/web/src/routes/nodes/[id]/+page.svelte)
- Added focused coverage proving the new metadata survives both core loading and server serialization:
  - [crates/dm-core/src/node/tests.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/node/tests.rs)
  - [crates/dm-server/src/tests.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-server/src/tests.rs)

### Validation

- `cargo test -p dm-core node::tests::test_node_status_preserves_dm_capability_bindings -- --nocapture` passed.
- `cargo test -p dm-server node_status_returns_dm_capability_bindings -- --nocapture` passed.
- `cargo check` passed for the workspace.
- `npm run check` passed in `web/`.
- After restarting the local dev stack, the running API now returns typed DM bindings for the pilot nodes:
  - `GET /api/nodes/dm-text-input`
  - `GET /api/nodes/dm-display`
- The rebuilt node detail surface now contains the new `DM Plane` / `Capability bindings` section in the shipped app bundle.

### Remaining issues

- This round formalizes the DM plane, but it does not yet remove node-local HTTP/WS code from the widget/display nodes.
- The graph editor still does not visualize capability bindings as first-class off-canvas relationships.
- Legacy third-party nodes may still carry the old ad hoc `interaction` shape until a broader migration pass exists.

### Next focus

- Start the next pilot round from this contract:
  choose one narrow runtime path where a declared DM binding can replace node-local protocol glue rather than only document it.

## Round 17

### Goal

Move the capability-binding pilot from metadata-only truth into the first real hidden-bridge runtime path, centered on `interaction-demo`.

### Tester path

1. Rebuild and restart the local server against the current branch so it can parse the converged `capabilities` schema.
2. Reinstall the affected builtin nodes:
   - `dm-bridge`
   - `dm-text-input`
   - `dm-display`
   - companion widget nodes
3. Start `tests/dataflows/interaction-demo.yml`.
4. Inspect the transpiled run artifact to confirm hidden bridge injection.
5. Query `/api/runs/:id/interaction`, `/api/runs/:id/messages/snapshots`, and `/api/runs/:id/messages`.
6. Send one web-style `input` message directly to the run API as a narrow bridge smoke test.

### Findings

- The old running `dm-server` could not parse the converged metadata shape and returned `500 Failed to parse node metadata` for `/api/nodes/dm-text-input` until it was restarted on the current branch.
- After restart, the live node API correctly reflected the converged model:
  `capabilities` now carries structured binding objects and top-level `dm` is `null`.
- Starting `interaction-demo` produced the expected hidden transpile result:
  the run's `dataflow.transpiled.yml` included:
  - injected env on `prompt` (`DM_BRIDGE_INPUT_PORT`)
  - injected env on `display` (`DM_BRIDGE_OUTPUT_PORT`)
  - one hidden `__dm_bridge` node
  - bridge wiring from `display/dm_bridge_output_internal` and to `prompt/dm_bridge_input_internal`
- The builtin runtime migration also landed:
  `dm-text-input`, `dm-button`, `dm-slider`, `dm-input-switch`, and `dm-display` now use the dora Node SDK path only, while DM-plane transport moved into the new `dm-bridge` node.
- The first real runtime dogfood pass did not yet complete the interaction loop:
  even with the hidden bridge injected, `/api/runs/:id/interaction` remained `{ "inputs": [], "streams": [] }`, and directly POSTing a web-style `input` message only persisted the input record itself without producing echoed display output.

### Fixes shipped

- Converged DM bindings into structured `capabilities` across the node model, API contract, docs, node manifests, and node detail page.
- Added hidden bridge lowering in transpile:
  - [crates/dm-core/src/dataflow/transpile/bridge.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/bridge.rs)
  - [crates/dm-core/src/dataflow/transpile/mod.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/mod.rs)
  - [crates/dm-core/src/dataflow/transpile/passes.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/passes.rs)
- Added the builtin hidden bridge node:
  - [nodes/dm-bridge/dm.json](/Users/yangchen/Desktop/dora-manager/nodes/dm-bridge/dm.json)
  - [nodes/dm-bridge/pyproject.toml](/Users/yangchen/Desktop/dora-manager/nodes/dm-bridge/pyproject.toml)
  - [nodes/dm-bridge/dm_bridge/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-bridge/dm_bridge/main.py)
- Migrated builtin widget/display nodes off node-local DM-plane transport code and onto the hidden-bridge dora-port path:
  - [nodes/dm-text-input/dm_text_input/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-text-input/dm_text_input/main.py)
  - [nodes/dm-button/dm_button/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-button/dm_button/main.py)
  - [nodes/dm-slider/dm_slider/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-slider/dm_slider/main.py)
  - [nodes/dm-input-switch/dm_input_switch/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-input-switch/dm_input_switch/main.py)
  - [nodes/dm-display/dm_display/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-display/dm_display/main.py)

### Validation

- `cargo test -p dm-core node::tests -- --nocapture` passed.
- `cargo test -p dm-core transpile -- --nocapture` passed.
- `cargo test -p dm-server node_status_returns_structured_capabilities_for_bindings -- --nocapture` passed.
- `pnpm -C web check` passed with 0 errors and 0 warnings.
- `python3 -m py_compile` passed for:
  - `dm-bridge`
  - `dm-display`
  - `dm-text-input`
  - `dm-button`
  - `dm-slider`
  - `dm-input-switch`
- `cargo run -p dm-cli -- node install dm-bridge dm-text-input dm-display dm-button dm-slider dm-input-switch` succeeded.
- A real `cargo run -p dm-cli -- start tests/dataflows/interaction-demo.yml` pass launched a run and confirmed the hidden bridge node was present in the transpiled YAML.

### Remaining issues

- The first runtime bridge dogfood pass is still incomplete:
  the live run did not register widgets or streams into `/api/runs/:id/interaction`, and a direct web-style input message did not traverse the full bridge path back to display output.
- `dm up` / `dm status` also behaved inconsistently in this environment:
  `dm up` reported success, while `dm status` still claimed the coordinator/daemon was not running.
- Because the real runtime path is not yet truthful, this round should not be treated as a finished bridge rollout even though schema convergence, transpile lowering, and node migration are in place.

### Next focus

- Diagnose the live hidden-bridge runtime gap on `interaction-demo` until one real run shows registered inputs, accepted user input, and echoed display output end to end.

## Round 18

### Goal

Turn `interaction-demo` from “hidden bridge exists in transpiled YAML” into a live, repeatable runtime path with at least widget registration proved and input routing narrowed to one remaining fault line.

### Tester path

1. Start the local product using `./dev.sh`.
2. Reinstall the current branch versions of `dm-bridge` and the migrated builtin widget/display nodes.
3. Clear stale Dora node processes and stop stale runs when runtime state becomes inconsistent.
4. Re-run `tests/dataflows/interaction-demo.yml` until a fresh run reaches `observed_nodes = 4`.
5. Check `/api/runs/:id/interaction`, run logs, and `interaction.db`.
6. Post one web-style `input` message directly to the live run API as the narrowest possible interaction smoke test.

### Findings

- `./dev.sh` is the right top-level launcher, but on this machine it can still collide with an already-running `dm-server` on `127.0.0.1:3210`; the script then reports both servers running even though its own `cargo run -p dm-server` child panicked on `AddrInUse`.
- The environment/runtime state was materially dirty at the start of the round:
  old `dm-text-input`, `dm-display`, and `dora-echo` processes from earlier runs were still resident even when `dm status` said the coordinator/daemon was not running.
- After clearing those stale node processes and starting a fresh run, the live hidden-bridge path improved:
  `/api/runs/:id/interaction` showed one registered `prompt` widget, `interaction.db` contained the `prompt/widgets` snapshot, and `__dm_bridge.log` recorded `emitted widgets for prompt`.
- A posted web-style `input` message updated `current_values.value` in `/api/runs/:id/interaction`, proving the server-side input record path is live.
- The end-to-end bridge/dataflow loop is still incomplete:
  no `routed input` bridge log appeared, `prompt.log` never showed `received bridge payload`, `display.log` never emitted a relay log, and `/api/runs/:id/interaction` continued to show `streams: []`.
- Runtime startup is still inconsistent even after cleanup:
  some fresh `dm start` attempts create a run record but come up with `observed_nodes = 0` and no `logs/` directory at all, while others launch normally with four observed nodes.
- Browser-first dogfooding was attempted, but the current desktop environment blocked both available tools:
  Computer Use could not obtain permission for Chrome in this thread, and Playwright failed locally because it tries to create `/.playwright-mcp` on a read-only root filesystem.

### Fixes shipped

- Updated [nodes/dm-bridge/dm_bridge/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-bridge/dm_bridge/main.py) so bridge event reception no longer depends on the main loop interleaving `node.next(timeout=...)` with HTTP polling:
  a background receiver thread now pulls Dora node events into a queue while the main loop continues polling `input` messages and routing outputs.
- Added extra bridge diagnostics to make the remaining live failure observable:
  startup now logs discovered widget/display specs, and the input-poll path logs when it actually sees input messages.

### Validation

- `python3 -m py_compile nodes/dm-bridge/dm_bridge/main.py` passed after the threading/logging update.
- `cargo run -p dm-cli -- node install dm-bridge` succeeded after the bridge update.
- A fresh live run (`82323686-fcea-4680-b717-1d1296e0a52c`) reached `observed_nodes = 4`, produced logs, and registered the prompt widget into `/api/runs/:id/interaction`.
- Posting a live web-style input message returned `{"seq":2}` and updated `current_values.value` for the prompt binding.

### Remaining issues

- The true bridge return path is still not proven:
  the live run still does not show any bridge input-routing log, prompt forwarding log, display relay log, or stream snapshot.
- Dora runtime lifecycle remains nondeterministic in this environment; the same `dm start` sequence alternates between healthy four-node runs and `observed_nodes = 0` runs with no logs.
- Because browser automation was blocked by the local desktop environment, this round still falls short of the intended “direct browser delivery” acceptance bar even though the runtime/API path was exercised as far as tooling allowed.

### Next focus

- Make Dora runtime startup deterministic enough that one fresh `interaction-demo` run always reaches a live four-node state with logs.
- Use the new bridge diagnostics to determine whether the missing path is:
  - bridge never polling live input messages
  - bridge polling but not matching the target spec
  - Dora hidden-edge delivery failing after `send_output`
- Once that path works, immediately re-run the browser surface and update this log with a true end-to-end input/display success.

## Round 19

### Goal

Convert the narrowed bridge diagnosis into a live end-to-end fix, while refusing to trust misleading runtime samples that do not correspond to real fresh node processes.

### Tester path

1. Use the user's manual browser dogfood result as the source of truth for the current front-half behavior.
2. Re-run `interaction-demo` repeatedly until a sample produces both fresh log files and fresh child processes.
3. Inspect bridge/runtime logs for the exact run ID rather than relying only on `/api/status`.
4. Patch the bridge/runtime execution model, reinstall the affected nodes, and re-run the same narrow sample.

### Findings

- The user's manual dogfooding round proved these live facts on run `b67adf97-05dc-46ec-8380-47b205fea36c`:
  - the browser showed the `Prompt` widget
  - submitting text updated `/api/runs/:id/messages?tag=input`
  - `current_values.value` changed to the submitted value
  - `streams` remained empty
- Reading the corresponding run logs removed the remaining ambiguity:
  `__dm_bridge.log` showed `polled 1 input message(s)` and `routed input -> prompt/value`, so the input bridge already reached `send_output`; the failure was further downstream at Dora delivery into `prompt`.
- The original single-process bridge model remained unstable:
  mixing `node.next(...)` with HTTP polling in one Python bridge process produced runs where the bridge exited early or stopped making forward progress without useful logs.
- A port-renaming attempt did not fix delivery:
  hidden internal ports were renamed away from `__...`, but healthy runs still stopped after widget registration and never produced prompt/display relay logs.
- A more structural change was then attempted:
  transpile now injects two hidden bridge instances instead of one:
  - `__dm_bridge_input` for `dm-server input -> dora output`
  - `__dm_bridge_display` for `dora input -> dm-server snapshot`
- That split did improve architectural truthfulness in one key way:
  a fresh live run reached `observed_nodes = 5`, and each bridge instance produced its own dedicated log file.
- Even after the split, runtime lifecycle remained the dominant blocker:
  some new runs still reused stale earlier child processes, some produced empty fresh log files, and process reality no longer reliably matched the latest run record.

### Fixes shipped

- Renamed injected internal bridge port IDs away from double-underscore names:
  - `dm_bridge_input_internal`
  - `dm_bridge_output_internal`
  - `dm_bridge_to_<yaml_id>`
  - `dm_display_from_<yaml_id>`
- Updated transpile lowering so bridge responsibilities can be split into two hidden instances:
  - [crates/dm-core/src/dataflow/transpile/bridge.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/bridge.rs)
  - [crates/dm-core/src/dataflow/transpile/passes.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/passes.rs)
- Refactored [nodes/dm-bridge/dm_bridge/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-bridge/dm_bridge/main.py) so input bridging and display relaying can run in separate bridge instances rather than competing inside one mixed event loop.
- Added extra runtime diagnostics around bridge startup and input polling to make live samples easier to classify.

### Validation

- `cargo test -p dm-core transpile -- --nocapture` passed after the split-bridge transpile changes.
- `python3 -m py_compile nodes/dm-bridge/dm_bridge/main.py` passed after each bridge runtime refactor.
- `cargo run -p dm-cli -- node install dm-bridge` succeeded after each bridge runtime refactor.
- A fresh run after the split (`681920f0-c84b-4bd4-b2df-93a23c8a6e00`) reached `observed_nodes = 5` and produced separate `__dm_bridge_input.log` and `__dm_bridge_display.log` files.

### Remaining issues

- End-to-end delivery is still not complete:
  no fresh run in this round reached `prompt.log` receipt, `display.log` relay, or a non-empty `streams` array.
- The most serious blocker is now Dora runtime lifecycle truth:
  several runs showed stale prior child processes surviving across supposedly fresh runs, or produced fresh run records without corresponding fresh child process/log evidence.
- Because that runtime state is not trustworthy yet, this round still cannot claim direct delivery of the hidden-bridge path.

### Next focus

- Fix or at least decisively characterize the Dora runtime stale-process / stale-log behavior around repeated `dm start` calls.
- Only after that, continue on the narrowed bridge path:
  `__dm_bridge_input send_output -> prompt receive -> echo -> display -> __dm_bridge_display snapshot`.

## Round 20

### Goal

Replace the experimental Python hidden bridge with a CLI-managed bridge path inspired by the old `dm-panel` design, and then push `interaction-demo` as far as possible toward a real end-to-end echoed stream.

### Tester path

1. Read the old `dm-panel` implementation and design notes to recover the original runtime boundary.
2. Move hidden bridge execution from `nodes/dm-bridge` toward `dm-cli bridge serve`.
3. Re-run `interaction-demo` on fresh samples, validating the actual spawned command, run logs, and `/interaction` state at each step.
4. When `dm up` / `dm start` proved unreliable, start Dora coordinator and daemon manually and continue live testing from there.

### Findings

- The old `dm-panel` code confirmed the right long-term boundary:
  bridge/panel behavior belongs on the `dm-cli` / runtime side, not inside a long-lived Python node package.
- CLI-managed hidden bridge injection is now live in transpiled YAML:
  fresh runs showed `__dm_bridge_input` and `__dm_bridge_display` launching as `/Users/yangchen/Desktop/dora-manager/target/debug/dm bridge serve ...`.
- The branch regained real live truth on one sample:
  with manually started Dora coordinator/daemon, `interaction-demo` reached `observed_nodes = 5`, `/api/runs/:id/interaction` showed the `Prompt` widget again, and both hidden bridge processes existed at the same time.
- Capturing raw `DORA_NODE_CONFIG` settled one major uncertainty:
  Dora passes a valid node-scoped config object to the CLI bridge, so `init_from_env()` is the correct initialization path for the runtime-managed bridge.
- The first fresh failure after that was concrete:
  `__dm_bridge_display` died with `database is locked` during `interaction.db` startup contention.
- After moving schema initialization away from immediate open and reworking input polling, the run was still not delivered end to end:
  a later sample wrote the browser `input` message and updated `current_values`, but `streams` stayed empty and the runtime remained fragile across repeated runs.
- Dora runtime bootstrap is still a separate blocker:
  `dm up` / `dm start` can still lose coordinator+daemon immediately in this environment, while manual `dora coordinator` + `dora daemon` sessions are able to sustain a live test long enough to gather signal.

### Fixes shipped

- Added a CLI-managed bridge implementation at [crates/dm-cli/src/bridge.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/bridge.rs).
- Added hidden CLI bridge subcommands in [crates/dm-cli/src/main.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/main.rs).
- Changed hidden bridge lowering to launch `dm bridge serve` from transpile:
  - [crates/dm-core/src/dataflow/transpile/bridge.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/bridge.rs)
  - [crates/dm-core/src/dataflow/transpile/passes.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/passes.rs)
- Fixed the bridge payload shape so input bridge messages are wrapped as JSON objects expected by widget nodes.
- Added raw node-config debug dumps for both hidden bridge instances to make Dora runtime truth inspectable.
- Reduced SQLite startup contention by delaying schema initialization until first write.
- Switched workspace `reqwest` to include blocking support and reworked bridge input polling accordingly.

### Validation

- `cargo build -p dm-cli` passed repeatedly after the CLI bridge changes.
- `cargo test -p dm-core transpile -- --nocapture` remained green after launcher lowering changes.
- A fresh run (`aff4a38f-...`) proved the hidden bridge launcher path was correct in `dataflow.transpiled.yml`:
  `path: /Users/yangchen/Desktop/dora-manager/target/debug/dm` and `args: bridge serve ...`.
- A fresh run (`2a45b401-...`) reached:
  - `runtime_running = true`
  - `observed_nodes = 5`
  - visible `Prompt` widget in `/api/runs/:id/interaction`
  - both CLI bridge processes present in `ps`
- Raw node config was successfully captured in:
  - `/Users/yangchen/.dm/runs/.../logs/bridge-input.node-config.yaml`
  - `/Users/yangchen/.dm/runs/.../logs/bridge-display.node-config.yaml`

### Remaining issues

- Full end-to-end delivery still did not land this round:
  no sample reached `prompt` forward logs, `display` relay logs, or a non-empty `streams` array.
- Runtime bootstrap through `dm up` / `dm start` remains unreliable enough that manual Dora startup was still required for the best live sample.
- The bridge runtime still needs one more narrowing pass around:
  - input polling visibility after the web message is inserted
  - display bridge write/read coordination after startup

### Next focus

- Keep the CLI-managed bridge as the only active long-term direction.
- Make one runtime bootstrap path reproducible without manual Dora sessions.
- Then finish the last narrow loop:
  `web input -> __dm_bridge_input -> prompt -> echo -> display -> __dm_bridge_display -> interaction stream`.
