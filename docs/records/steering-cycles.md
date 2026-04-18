# Steering Cycles

## Cycle 1

### Current project judgment

- Round 1 repaired startup guidance and stop semantics enough to reduce obvious trust breaks.
- The project is still in proof-of-path stage.
- The remaining highest-risk area is the first visible web path, especially Dashboard-first behavior.

### Options considered

- Keep polishing dashboard history and shortcut presentation
- Expand editor or node capabilities
- Harden one deterministic first-success path through the visible web product

### Recommendation

Harden one deterministic first-success path through the visible web product.

### Dissent / warning

- Do not mistake broader UI polish for real progress while the main path still has caveats.
- Do not widen feature scope before the first-success loop is provable end to end.

### Final decision

Round 2 will focus on a canonical first-success web path:

1. open Dashboard
2. click a known-good entry
3. start a real run
4. inspect live status
5. stop cleanly

To reduce dependence on noisy or stale history, Dashboard now gets a dedicated built-in `Hello Timer` entry point instead of relying only on frequent-dataflow cards.

### Acceptance condition

- A first-time user can reach a real run from the Dashboard through one obvious path without guessing.
- The run page clearly shows a healthy live state and can stop successfully.
- This path does not depend on stale history or missing saved workspaces.

## Cycle 2

### Current project judgment

- The canonical Dashboard path is now visually verified end to end.
- `Hello Timer` produces visible live output in the `Message` panel.
- The run detail no longer mislabels historical observed-node counts as active live nodes.
- The first-success problem is no longer the highest-risk unknown.

### Options considered

- Keep polishing Dashboard history and first-impression noise
- Stay on run-detail truthfulness and terminal semantics only
- Move one step deeper into the `edit -> rerun -> confirm change` builder loop

### Recommendation

Move one step deeper into the `edit -> rerun -> confirm change` loop using the now-stable `Hello Timer` path or another equally deterministic flow.

### Dissent / warning

- Do not widen into editor feature work or broader UI polish without proving one real edit loop first.
- Do not regress run-detail truthfulness while moving deeper into the builder workflow.

### Final decision

Round 4 will focus on a deterministic first modification loop:

1. start from a known-good flow
2. open the editable representation
3. make one obvious, low-risk change
4. rerun
5. confirm the changed behavior from the product itself

### Acceptance condition

- A first-time technical user can complete one small modification without guessing where to edit or how to rerun.
- The resulting behavior change is visible in the product, not only inferable from code.
- The path preserves the honesty of the already-fixed run lifecycle surfaces.

## Cycle 3

### Current project judgment

- The product now has a believable first-success path and a believable first modification loop.
- `Hello Timer` can be launched, inspected, edited from a saved workspace, rerun, and visually confirmed.
- The next highest-leverage weakness is no longer the core workflow; it is first-impression noise around that workflow.

### Options considered

- Keep deepening editor features
- Broaden demo or capability coverage
- Clean up Dashboard noise so the newly proven paths remain the obvious paths

### Recommendation

Return to the Dashboard and reduce misleading historical noise without disturbing the now-proven main loops.

### Dissent / warning

- Do not spend the next cycle on cosmetic polish that does not change what the user sees first.
- Do not regress the Dashboard quick-start and editable-workspace bridge while cleaning up history sections.

### Final decision

Round 5 will focus on Dashboard first-impression noise:

1. stale frequent-dataflow fallbacks
2. old failed runs dominating recent history
3. weak separation between proven current paths and legacy broken history

### Acceptance condition

- A first-time user sees the proven `Run Hello Timer` and `Edit Hello Timer` paths as the clearest next actions.
- Old broken history no longer competes with those paths for attention or trust.

## Cycle 4

### Current project judgment

- The homepage now behaves like a real starting surface: quick-start is obvious, saved workspaces are the only frequent shortcuts, and old `runtime_lost` noise no longer dominates the first view.
- The product has already proven two optimistic paths: first success and first modification.
- The next highest-risk unknown is no longer discovery; it is recovery.

### Options considered

- Keep tightening Dashboard history bounds even further
- Re-run the already-proven `edit -> rerun` loop again
- Validate one deterministic `failure -> diagnose -> fix -> rerun` path through the product

### Recommendation

Validate one deterministic `failure -> diagnose -> fix -> rerun` path through the product.

### Dissent / warning

- Do not widen this into generic lifecycle polish or broad error-taxonomy cleanup.
- Do not leave the product on a broken workspace path without proving the recovery path back to green.

### Final decision

Round 6 will focus on first recovery:

1. start from a known-good editable workspace
2. introduce one controlled, low-risk failure
3. observe how the product reports the failure
4. fix it through the normal editing path
5. rerun and confirm the product is healthy again

### Acceptance condition

- A first-time technical user can trigger one realistic failure, understand what broke from the product itself, apply one targeted fix, rerun, and get back to a healthy run without leaving the visible product workflow.

## Cycle 5

### Current project judgment

- The product now has a believable recovery loop for one controlled workspace failure.
- Save-time workspace truth is now immediate: broken workspaces disable `Run`, restored workspaces become runnable again without a page refresh.
- The next highest-leverage weakness is not whether recovery is possible, but whether broken workspaces explain themselves well enough before the user attempts to run them.

### Options considered

- Expand recovery coverage to more failure classes immediately
- Return to broader Dashboard or Runs-page polish
- Make invalid or incomplete workspaces self-explanatory in-place

### Recommendation

Make broken workspaces self-explanatory in-place.

### Dissent / warning

- Do not broaden into a generic error-center redesign.
- Do not assume that a red badge alone is enough diagnosis for first-time technical users.

### Final decision

Round 7 will focus on workspace-side diagnosis clarity:

1. when a workspace is invalid or missing nodes, say what is wrong
2. say what the user should do next
3. keep the user in the workspace page instead of forcing trial-and-error runs or console inspection

### Acceptance condition

- A first-time technical user can open a broken workspace and understand both the cause and the next repair step from the page itself, before attempting another run.

## Cycle 6

### Current project judgment

- Missing-node recovery is now both truthful and legible on the workspace page.
- The next recovery gaps are narrower and more specific: syntax errors and failures that only show up once the user clicks `Run`.
- The project no longer needs a generic recovery push; it needs coverage of the remaining failure classes that are still ambiguous.

### Options considered

- Return to broader navigation or dashboard polish
- Expand immediately into many recovery modes at once
- Cover the next two concrete broken states: `invalid_yaml` and post-click run failures

### Recommendation

Cover the next two concrete broken states: `invalid_yaml` and post-click run failures.

### Dissent / warning

- Do not widen this into a generic diagnostics subsystem.
- Do not leave missing-node recovery unfinished, but also do not overfit the product to that one class of error.

### Final decision

Round 8 will focus on the next unresolved recovery surfaces:

1. an invalid YAML workspace before run
2. a run-start or runtime failure that still passes workspace executable preflight
3. whether the page keeps those failures visible and actionable long enough for the user to recover

### Acceptance condition

- A first-time technical user can understand and act on both a syntax-level workspace failure and a run-time failure without relying on browser refreshes, console output, or code inspection.

## Cycle 7

### Current project judgment

- The product now covers three recovery layers:
  - missing nodes before run
  - invalid YAML before run
  - post-click run-start failure that returns to the workspace page
- The remaining weakness is no longer visibility, but message quality.
- The product tells the truth, but the post-click failure truth is still too raw.

### Options considered

- Broaden recovery coverage again
- Return to broader navigation polish
- Make run-failure messaging shorter, cleaner, and more actionable

### Recommendation

Make run-failure messaging shorter, cleaner, and more actionable.

### Dissent / warning

- Do not strip so much detail that technical users lose the actual cause.
- Do not keep dumping backend stack frames directly into the primary workspace recovery surface.

### Final decision

Round 9 will focus on run-failure message quality:

1. preserve the root cause
2. suppress obvious backend noise and stack-trace repetition
3. keep the workspace page readable enough that recovery still feels productized

### Acceptance condition

- A first-time technical user can read a failed run banner on the workspace page, understand the cause quickly, and know what to do next without parsing a wall of backend trace output.

## Cycle 8

### Current project judgment

- The workspace page now covers both pre-run and post-click failure classes with truthful persistence.
- The remaining weakness in the core recovery path is no longer missing visibility; it is the quality and layering of the failure explanation.
- The product still risks feeling like an internal tool if the first failure surface is dominated by daemon and coordinator stderr.

### Options considered

- Expand into more recovery classes immediately
- Return to broader navigation or Dashboard polish
- Tighten run-failure message quality while preserving the technical cause

### Recommendation

Tighten run-failure message quality while preserving the technical cause.

### Dissent / warning

- Do not over-sanitize the error into vague product copy.
- Do not redesign the full failure subsystem just to improve one surface.

### Final decision

Round 9 will focus on structured run-failure messaging on the workspace page:

1. keep the root cause visible
2. remove trigger IDs, repeated trace noise, and `Location:` blocks from the primary surface
3. keep raw technical detail available on demand

### Acceptance condition

- A first-time technical user can read the workspace failure banner after a failed run attempt, understand what failed and what to do next, and only open raw detail if they want deeper diagnostics.

## Cycle 9

### Current project judgment

- The workspace page now explains post-click run failures cleanly enough for real recovery.
- However, failed runs still carry the raw backend `outcome_summary` into history surfaces.
- That inconsistency weakens trust: the product looks cleaned up in the workspace, then backend-shaped again in `Runs`.

### Options considered

- Move to a different surface such as Dataflows or broader navigation
- Refactor backend run outcome persistence
- Normalize failed-run summaries at the display layer across run-history surfaces

### Recommendation

Normalize failed-run summaries at the display layer across run-history surfaces.

### Dissent / warning

- Do not mutate stored run truth just to polish one page.
- Do not leave history surfaces speaking a different language than the workspace page.

### Final decision

Round 10 will align failed-run history surfaces with the new workspace failure messaging:

1. normalize `outcome_summary` display on homepage recent cards
2. normalize it on the `Runs` table
3. normalize it on run detail header surfaces

### Acceptance condition

- A first-time technical user sees the same concise failure explanation on the workspace page, homepage, runs history, and run detail header, without reintroducing raw trigger IDs or `Location:` traces.

## Cycle 10

### Current project judgment

- The quick-start path, edit loop, recovery loop, and failure wording are now much stronger than they were at the start of this effort.
- The next weakness is broader orientation after first success: once a user leaves the canonical demo path, the product still does not clearly tell them where to go next.
- `Dataflows` is the most important surface in that problem because it is the first place a user encounters the broader saved-workspace landscape.

### Options considered

- Broaden into generic onboarding copy across the whole app
- Rework `Dataflows` so it acts like a real workspace map and next-step surface
- Shift attention to a different area such as nodes or settings

### Recommendation

Rework `Dataflows` so it acts like a real workspace map and next-step surface.

### Dissent / warning

- Do not turn this into generic IA polish.
- Do not add onboarding theater without clarifying one real next-step path beyond `Hello Timer`.

### Final decision

Round 11 will focus on the `Dataflows` page as the first broad exploration surface:

1. explain what `Run` and `Workspace` mean
2. make recommended next workspaces obvious
3. separate ready workspaces from items that still need repair or extra setup

### Acceptance condition

- A first-time technical user can open `Dataflows` and understand which saved workspace to explore next, whether they are editing something persistent, and which items are not ready yet.

## Cycle 11

### Current project judgment

- The broader first-success path is now materially stronger, but direct human feedback exposed a different high-friction surface: the node catalog itself.
- The current `Nodes` page still miscommunicates what it is, hides important source and install distinctions, and leaves newly created nodes without a clear next step.
- This is no longer a speculative product concern; it is confirmed by real user feedback.

### Options considered

- Stay on the interaction-demo second-task loop
- Pause broader flow work and insert a node-catalog repair round
- Attempt a large multi-surface onboarding rewrite

### Recommendation

Insert a node-catalog repair round now.

### Dissent / warning

- Do not redesign the entire node system in one pass.
- Do not let the fix drift into purely cosmetic card polish; the point is orientation, findability, and next-step clarity.

### Final decision

Round 12 will focus on the `Nodes` catalog and node detail entry path:

1. stop calling the page an installed-only list when it is really a mixed catalog
2. expose source, install status, and maintainers clearly on cards
3. make new nodes land in a useful “continue editing” state
4. restore support for `display.avatar` in the UI

### Acceptance condition

- A user can tell which nodes are builtin, local, or imported, filter the catalog down to what they need, and create a node without losing the next editing step.

## Cycle 12

### Current project judgment

- The node-catalog repair round landed cleanly, so the earlier interruption can now return to the deferred `interaction-demo` second-task flow.
- The runtime and interaction protocol are not the current problem; direct API validation shows interactive inputs and echoed output already work.
- The remaining weakness is product guidance: a first-time user is not clearly told how to submit input or where to watch the result.

### Options considered

- Deepen node-catalog work immediately again
- Debug interaction runtime plumbing as if the protocol were broken
- Tighten the interactive run surface so the next-step path is explicit

### Recommendation

Tighten the interactive run surface so the next-step path is explicit.

### Dissent / warning

- Do not overfit the product around one demo-specific flow.
- Do not keep treating this as a backend bug when the evidence points to discoverability and affordance.

### Final decision

Round 13 will focus on interactive run guidance:

1. make the `Input` -> `Message` loop explicit on the run page
2. make submit controls visually obvious for first-time users
3. keep the change narrow to the interactive surface

### Acceptance condition

- A first-time technical user can open an interactive run and immediately understand where to enter a value, how to submit it, and where the response will appear.

## Cycle 13

### Current project judgment

- The interactive run path is now legible enough; direct user feedback confirms the remaining friction is not “what do I do here?” but “can I trust lifecycle transitions once I do it?”
- Two concrete gaps surfaced together:
  - the interactive hint should stop occupying permanent space after it has done its job,
  - `Stop` is still semantically weak because it does not survive refresh or navigation and gives no trustworthy background-drain model.
- This is a product-truth problem, not just a copy problem. A long-lived runtime cannot present `running` after a stop request has already been accepted.

### Options considered

- Keep deepening the interaction-demo submission loop only
- Pause and repair run-stop lifecycle truthfulness
- Expand into broader run-history or recovery polish

### Recommendation

Pause and repair run-stop lifecycle truthfulness, while also letting the interaction hint dismiss itself cleanly.

### Dissent / warning

- Do not claim that stop is “fast” if the underlying runtime still needs time to drain.
- Do not solve this only with local frontend state; the stop request must become a persisted fact that survives reloads.

### Final decision

Round 14 will focus on stop lifecycle trust:

1. make the interactive guidance dismissible and persistent
2. persist stop-request state in run metadata
3. surface a real `stopping` UI state across refresh/navigation
4. explicitly tell users they can leave safely while the stop drains in the background
5. tighten the synchronous wait budget so `dm` stops pretending it needs to block for a full backend timeout before admitting the run is still draining

### Acceptance condition

- A user can dismiss the interactive hint and keep it dismissed on reload.
- After clicking `Stop`, the run page survives refresh/navigation with a visible `stopping` state and leave-safe copy.
- The run API exposes the stop request while the runtime is still draining, and clears it once the run reaches a terminal state.

## Cycle 14

### Current project judgment

- Stop-state truth is now repaired, so the next bottleneck is no longer UI semantics; it is real shutdown latency on interactive flows.
- Direct measurement shows this is not a universal runtime problem:
  `demo-hello-timer` stops almost instantly, while `interaction-demo` was taking about 11 to 12 seconds.
- Process observation isolates the slow path to dm-managed interactive widget nodes, especially `dm-text-input`, rather than to the entire dataflow or Dora runtime in general.

### Options considered

- Treat slow stop as a generic Dora/runtime limitation and stop there
- Keep investigating until the blocking node or node class is isolated
- Attempt a broad lifecycle redesign before understanding the hotspot

### Recommendation

Keep investigating until the blocking node class is isolated, then apply the smallest fix at the node layer first.

### Dissent / warning

- Do not overgeneralize from one interactive flow to all shutdown behavior.
- Do not force a runtime-wide contract change if the problem is localized to dm widget nodes that can already observe dm stop state.

### Final decision

Round 15 will focus on stop-latency diagnosis and the first narrow fix:

1. compare interactive and non-interactive stop times
2. identify which process stays alive during slow stops
3. test whether that node can exit faster when it sees dm's persisted stop request
4. validate whether the user-facing dm stop path becomes materially faster

### Acceptance condition

- The investigation names the blocking node or node class.
- There is at least one concrete, validated explanation for the old 11 to 12 second stop time.
- If a bounded fix exists in dm-owned nodes, validate whether it reduces dm-driven stop latency significantly.
