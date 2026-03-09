# Backend Health Roadmap To A-

## Purpose

This document defines the concrete execution plan for moving the backend from its current `C+` state to `A-`.

It is intended for both humans and agents.

It answers four questions:

- What does `A-` mean in this repository?
- What is the current gap?
- In what order should the work happen?
- How should each phase be executed and validated?

This roadmap should be read together with:

- [backend_code_health_standard.md](./backend_code_health_standard.md)
- [backend_structure_health_improvement_plan.md](./backend_structure_health_improvement_plan.md)

## Current Baseline

As of the latest health pass, the backend is approximately `C+`.

What is already true:

- `cargo clippy --workspace --all-targets -- -D warnings` passes
- `cargo test --workspace` passes
- recent `runs` lifecycle defects were fixed
- `crates/dm-core/src/runs/service.rs` was split into focused submodules
- `runs` service coverage improved materially

What still blocks `A-`:

- `dm-core` total coverage is still below the repository baseline
- several high-risk modules are still weakly tested
- some high-risk modules remain below `35%` line coverage
- some modules are still structurally dense even though the worst hotspot was reduced

## Target Definition

`A-` in this repository means:

- strict workspace lint passes
- workspace tests pass
- `dm-core` total line coverage reaches at least `70%`
- `dm-server` total line coverage reaches at least `60%`
- actively used high-risk modules are mostly at or above `50%`
- no critical high-risk module remains near-zero coverage
- no major backend regression hotspot is known and untested

`A-` is not the same as `A`.

For this roadmap, `A-` allows:

- a small number of medium-risk modules in the `35-49%` band
- a few larger files if their boundaries are clear and they are well-tested

It does not allow:

- `0-20%` coverage in critical import/install/runtime paths
- red gates
- major behavior living behind untested external-process branches

## Primary Gap

The current main problem is no longer baseline gate failure.

The main problem is concentrated risk in a few high-risk modules:

- `crates/dm-core/src/dataflow/import.rs`
- `crates/dm-core/src/node/import.rs`
- `crates/dm-core/src/node/install.rs`
- `crates/dm-core/src/install/github.rs`
- `crates/dm-core/src/install/source.rs`
- selected `dm-server` handlers if coverage drops below `50%`

These modules are risky because they:

- mutate filesystem layout
- shell out to external commands
- depend on Git or Dora CLI behavior
- import remote content
- create persisted state consumed by later features

## Execution Rules

Every agent working from this roadmap should follow these rules.

### Rule 1. Keep Gates Green

Before and after each phase, run:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

If either fails:

- stop feature expansion
- fix or revert the current phase
- do not move to the next phase

### Rule 2. Prefer Behavior Tests Before Deep Refactors

If a high-risk module is weakly tested:

- first add or improve tests for externally observable behavior
- only then split or refactor internals

Exception:

- if the file is so large or tangled that tests cannot be written cleanly, do a minimal structural split first

### Rule 3. Optimize For High-Risk Coverage, Not Vanity Coverage

Do not chase easy coverage in low-risk model or serialization code while import/install/runtime paths remain weak.

### Rule 4. Avoid Broad Churn

For each phase:

- touch the smallest module set that closes the target risk
- avoid unrelated style edits
- preserve public APIs unless explicitly planned

### Rule 5. Record New Baselines

After each completed phase, update the measured baseline in the working notes or PR description:

- crate total coverage
- touched module coverage
- files split or stabilized

## Phase Plan

### Phase 1. Raise High-Risk Import Coverage

Objective:

- turn remote and local import paths from weakly protected to deterministic and reviewable

Primary targets:

- `crates/dm-core/src/dataflow/import.rs`
- `crates/dm-core/src/node/import.rs`

Required work:

1. Add deterministic tests for GitHub URL parsing variants:
   - repo root
   - `tree/<ref>/<dir>`
   - `blob/<ref>/<file>`
   - invalid URL formats
2. Add tests for local import behavior:
   - missing source
   - duplicate destination
   - single YAML file import
   - directory import with config/meta copy
   - multiple YAML rejection
3. Add tests for cleanup behavior on failed clone/copy paths
4. If needed, split parsing and copy helpers into small pure functions

Exit criteria:

- `dataflow/import.rs` line coverage >= `50%`
- `node/import.rs` line coverage >= `50%`
- no import path relies on untested URL interpretation

Suggested commands:

- `cargo test -p dm-core tests::tests_dataflow`
- `cargo test -p dm-core tests::tests_node`
- `cargo llvm-cov -p dm-core --summary-only --ignore-filename-regex 'tests/'`

### Phase 2. Raise Install Coverage

Objective:

- protect install flows that currently depend too heavily on external systems

Primary targets:

- `crates/dm-core/src/node/install.rs`
- `crates/dm-core/src/install/github.rs`
- `crates/dm-core/src/install/source.rs`
- `crates/dm-core/src/install/mod.rs`

Required work:

1. Add tests for install precondition failures:
   - unsupported build
   - missing source
   - invalid metadata
   - missing tools
2. Add tests for local branches before network execution:
   - path validation
   - duplicate install handling
   - invalid archive / invalid source behavior
3. Isolate external command construction into helper functions when useful
4. Prefer fake command outputs and temp directories over real network calls

Exit criteria:

- `node/install.rs` line coverage >= `35%`
- `install/github.rs` line coverage >= `35%`
- `install/source.rs` line coverage >= `35%`
- install behavior failures are locally reproducible in tests

Suggested commands:

- `cargo test -p dm-core install::tests`
- `cargo test -p dm-core node::tests`
- `cargo llvm-cov -p dm-core --summary-only --ignore-filename-regex 'tests/'`

### Phase 3. Push `dm-core` To Baseline

Objective:

- move `dm-core` from sub-baseline to healthy-for-iteration

Primary targets:

- crate total coverage
- remaining weak runtime/dataflow support paths

Required work:

1. Re-run coverage after Phases 1 and 2
2. Rank remaining weak modules by:
   - risk
   - change frequency
   - distance from baseline
3. Add targeted tests only for top-ranked gaps
4. Split any file that is still both:
   - above `500` lines
   - hard to test because concerns remain mixed

Exit criteria:

- `dm-core` total line coverage >= `60%`
- no critical high-risk submodule remains below `20%`
- most active high-risk `dm-core` modules >= `35%`

This phase is the minimum required to move cleanly out of `C+`.

### Phase 4. Push `dm-core` From Baseline To A- Range

Objective:

- close the gap between “healthy enough” and “strong”

Primary targets:

- `dm-core` total line coverage >= `70%`
- critical high-risk modules >= `50%`

Required work:

1. Continue with targeted tests for remaining runtime, dataflow, and install weak spots
2. Reduce side-effect density where a single function still:
   - validates input
   - reads files
   - calls external tools
   - mutates persistent state
3. Prefer splitting by responsibility:
   - parse / validate
   - prepare filesystem layout
   - execute external call
   - persist result
4. Add failure-path tests whenever a function maps external failures into persistent terminal state

Exit criteria:

- `dm-core` total line coverage >= `70%`
- `runs`, `dataflow`, and `install` active high-risk modules mostly >= `50%`

### Phase 5. Raise `dm-server` To A- Range

Objective:

- ensure the transport layer does not become the new weak point

Primary targets:

- `crates/dm-server/src/handlers/*.rs`

Required work:

1. Re-measure handler coverage
2. For any handler below `50%`, add tests for:
   - happy path
   - bad request path
   - not found path
   - backend error mapping path
3. Keep route contracts stable
4. Avoid large handler modules accumulating orchestration logic

Exit criteria:

- `dm-server` total line coverage >= `60%`
- actively used handler modules >= `50%`

Suggested commands:

- `cargo test -p dm-server`
- `cargo llvm-cov -p dm-server --summary-only --ignore-filename-regex 'tests/'`

## Recommended Order

Agents should use this default sequence:

1. `dataflow/import.rs`
2. `node/import.rs`
3. `node/install.rs`
4. `install/github.rs`
5. `install/source.rs`
6. re-measure `dm-core`
7. highest-risk remaining `dm-core` module
8. `dm-server` weak handlers

Reason:

- this order removes the lowest-coverage, highest-risk paths first
- it improves the overall grade faster than further polishing already decent modules

## Non-Goals

Do not spend roadmap budget on these before high-risk coverage is fixed:

- broad rename-only refactors
- style-only cleanup
- low-risk serialization coverage farming
- speculative architecture rewrites
- frontend or UX work

## Agent Task Template

When executing one phase, use this structure:

### 1. Scope

- target file(s)
- risk being reduced
- explicit coverage goal

### 2. Baseline

Run:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo llvm-cov -p dm-core --summary-only --ignore-filename-regex 'tests/'`
- or `cargo llvm-cov -p dm-server --summary-only --ignore-filename-regex 'tests/'`

### 3. Implement

- add behavior-first tests
- do minimal refactor required to make tests clean
- keep API stable

### 4. Verify

Run:

- targeted crate tests
- strict workspace lint
- strict workspace tests
- coverage measurement for the affected crate

### 5. Report

Always report:

- files changed
- previous and new module coverage
- previous and new crate coverage
- any residual risk not addressed

## Completion Criteria For A-

This roadmap is complete when all of the following are true:

- `cargo clippy --workspace --all-targets -- -D warnings` passes
- `cargo test --workspace` passes
- `dm-core` total line coverage >= `70%`
- `dm-server` total line coverage >= `60%`
- critical active high-risk modules are mostly >= `50%`
- no critical active high-risk module remains below `20%`
- no backend-critical file above `500` lines remains unsplit without justification

## Current Recommendation

If an agent starts from this document now, it should begin with:

1. `crates/dm-core/src/dataflow/import.rs`
2. `crates/dm-core/src/node/import.rs`
3. `crates/dm-core/src/node/install.rs`

That path is the fastest route from the current `C+` toward `B` and then `A-`.
