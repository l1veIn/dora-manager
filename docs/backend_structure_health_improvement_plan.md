# Backend Structure And Code Health Improvement Plan

## Purpose

This document defines the engineering plan for improving backend code structure, maintainability, and code health in the `dm-core` and `dm-server` crates.

The goal is to turn the current backend from "works and is improving" into "easy to maintain, test, review, and extend".

## Current Situation

The backend has improved materially:

- Targeted backend linting passes for `dm-core` and `dm-server`
- Core tests and server handler tests are in place
- Coverage has improved in several important modules

However, the backend still has structural and maintenance risks:

- Several Rust source files are too large and mix multiple responsibilities
- Coverage is uneven across critical modules
- Some modules are hard to review because domain logic, I/O, and orchestration are combined
- Workspace-wide quality gates are still blocked by non-backend issues in `dm-cli`

## Primary Engineering Goals

### Goal 1: Reduce File Size And Responsibility Creep

Refactor oversized backend files into smaller domain-oriented modules.

Target outcome:

- Most backend source files remain below 300 lines
- Files above 500 lines are eliminated unless strongly justified
- Each file has one clear responsibility

### Goal 2: Improve Maintainability And Change Safety

Make backend behavior easier to change without causing regressions.

Target outcome:

- Public APIs stay stable
- Internal implementations are split into isolated modules
- Tests live closer to the logic they validate
- Handler and service logic are easier to trace

### Goal 3: Raise Backend Code Health

Strengthen objective code health metrics.

Target outcome:

- `cargo clippy -p dm-core --all-targets -- -D warnings` passes
- `cargo clippy -p dm-server --all-targets --no-deps -- -D warnings` passes
- `cargo test -p dm-core` passes
- `cargo test -p dm-server` passes

### Goal 4: Raise Coverage In High-Risk Areas

Prioritize tests where backend failures are costly or easy to miss.

Target outcome:

- `dm-core` overall line coverage reaches at least 55%
- `dm-server` overall line coverage reaches at least 55%
- Critical modules should exceed these minimums:
  - `dm-server/src/handlers.rs` or its replacements: 70%+ line coverage
  - `dm-core/src/lib.rs` or split API modules: 60%+ line coverage
  - `dm-core/src/install.rs` or split install modules: 50%+ line coverage
  - `dm-core/src/events.rs` or split event modules: 65%+ line coverage

## Scope

### In Scope

- `crates/dm-core`
- `crates/dm-server`
- Tests directly supporting backend quality
- Internal module reorganization
- Public API preserving refactors

### Out Of Scope

- Frontend structure changes
- Large CLI UX redesign
- Product-level feature additions unrelated to backend maintainability
- Unrelated style churn

## Key Problems To Address

### Oversized Files

Current hotspots include:

- `crates/dm-core/src/lib.rs`
- `crates/dm-core/src/events.rs`
- `crates/dm-core/src/install.rs`
- `crates/dm-server/src/handlers.rs`

These files currently combine multiple responsibilities, which increases:

- review cost
- merge conflict frequency
- regression risk
- onboarding time

### Mixed Concerns

Several files currently mix:

- transport layer logic
- orchestration logic
- file system access
- subprocess execution
- serialization
- event logging

This makes behavior harder to reason about and harder to test in isolation.

### Uneven Test Coverage

Coverage has improved, but it remains inconsistent. Some critical logic is still under-tested, especially around:

- install flows
- orchestration entrypoints
- event-heavy paths
- runtime startup and shutdown behavior

## Target Module Structure

### `dm-server`

Refactor `crates/dm-server/src/handlers.rs` into a module tree:

- `crates/dm-server/src/handlers/mod.rs`
- `crates/dm-server/src/handlers/system.rs`
- `crates/dm-server/src/handlers/runtime.rs`
- `crates/dm-server/src/handlers/nodes.rs`
- `crates/dm-server/src/handlers/dataflow.rs`
- `crates/dm-server/src/handlers/events.rs`

Suggested responsibility split:

- `system.rs`: `doctor`, `versions`, `status`, `get_config`, `update_config`
- `runtime.rs`: `install`, `uninstall`, `use_version`, `up`, `down`
- `nodes.rs`: `list_nodes`, `node_status`, `install_node`, `uninstall_node`
- `dataflow.rs`: `run_dataflow`, `stop_dataflow`
- `events.rs`: `query_events`, `ingest_event`, `export_events`

### `dm-core`

Refactor `crates/dm-core/src/lib.rs` into API-oriented modules:

- `crates/dm-core/src/api/mod.rs`
- `crates/dm-core/src/api/doctor.rs`
- `crates/dm-core/src/api/version.rs`
- `crates/dm-core/src/api/runtime.rs`
- `crates/dm-core/src/api/setup.rs`

Keep `crates/dm-core/src/lib.rs` as the public facade:

- module declarations
- re-exports
- minimal glue only

Refactor `crates/dm-core/src/install.rs` into:

- `crates/dm-core/src/install/mod.rs`
- `crates/dm-core/src/install/github.rs`
- `crates/dm-core/src/install/binary.rs`
- `crates/dm-core/src/install/source.rs`
- `crates/dm-core/src/install/archive.rs`
- `crates/dm-core/src/install/progress.rs`

Refactor `crates/dm-core/src/events.rs` into:

- `crates/dm-core/src/events/mod.rs`
- `crates/dm-core/src/events/model.rs`
- `crates/dm-core/src/events/builder.rs`
- `crates/dm-core/src/events/store.rs`
- `crates/dm-core/src/events/export.rs`
- `crates/dm-core/src/events/op.rs`

## Implementation Strategy

Use incremental, non-breaking refactors. Do not attempt a full backend rewrite in one pass.

### Phase 1: Stabilize Quality Gates

Objective:

- Keep backend quality checks green while work continues

Steps:

1. Maintain passing backend lint targets:
   - `cargo clippy -p dm-core --all-targets -- -D warnings`
   - `cargo clippy -p dm-server --all-targets --no-deps -- -D warnings`
2. Maintain passing backend tests:
   - `cargo test -p dm-core`
   - `cargo test -p dm-server`
3. Capture current coverage baseline before each major refactor:
   - `cargo llvm-cov -p dm-core --summary-only`
   - `cargo llvm-cov -p dm-server --summary-only`

Exit criteria:

- Backend checks are green before structural changes begin

### Phase 2: Split `dm-server` Handlers

Objective:

- Reduce HTTP layer complexity with minimal behavioral risk

Steps:

1. Create `handlers/` module directory
2. Move route handlers into domain files
3. Keep route registration in `main.rs` unchanged except import paths
4. Keep request and response types near their relevant handler modules
5. Preserve existing tests and add module-local tests where useful

Exit criteria:

- `handlers.rs` is removed or reduced to a tiny `mod.rs`
- All existing `dm-server` tests still pass
- No API route changes

### Phase 3: Split `dm-core` Public API Layer

Objective:

- Separate public entrypoints by domain

Steps:

1. Create `api/` module tree under `dm-core`
2. Move `doctor` into `api/doctor.rs`
3. Move version-related functions into `api/version.rs`
4. Move runtime-related functions into `api/runtime.rs`
5. Move setup logic into `api/setup.rs`
6. Re-export existing public functions from `lib.rs`

Exit criteria:

- Public API signatures remain unchanged
- `lib.rs` becomes a small facade
- Existing tests still pass without caller changes

### Phase 4: Split `install` Internals

Objective:

- Isolate network, archive, subprocess, and progress concerns

Steps:

1. Move GitHub API structs and release fetch logic into `install/github.rs`
2. Move binary download logic into `install/binary.rs`
3. Move source build logic into `install/source.rs`
4. Move tar/zip extraction and binary discovery into `install/archive.rs`
5. Move progress emission into `install/progress.rs`
6. Keep `install/mod.rs` as the orchestration layer

Exit criteria:

- Each install file has a single concern
- Archive helpers and progress helpers have direct unit tests
- `install` behavior remains unchanged

### Phase 5: Split `events` Internals

Objective:

- Make event modeling, persistence, and export independently maintainable

Steps:

1. Move event data types into `events/model.rs`
2. Move builder into `events/builder.rs`
3. Move SQLite store into `events/store.rs`
4. Move XES export into `events/export.rs`
5. Move `try_emit` and `OperationEvent` into `events/op.rs`
6. Keep `events/mod.rs` as the public export surface

Exit criteria:

- Event system remains API-compatible
- Store logic and export logic are independently testable
- Event code review scope becomes smaller and clearer

### Phase 6: Raise Coverage In Priority Order

Objective:

- Improve safety where regressions are most likely

Priority order:

1. `dm-core` public API entrypoints
2. `dm-core` install paths
3. `dm-core` event store and event result paths
4. `dm-server` runtime and dataflow handlers
5. `dm-server` listener/bootstrap behavior where practical

Concrete steps:

1. Add deterministic unit tests for pure helpers and local file I/O
2. Add fake binary/script based tests for subprocess-driven logic
3. Prefer handler-level tests that do not bind ports
4. Use temporary directories for all backend stateful tests
5. Test both success and failure paths for any endpoint or API with side effects

Exit criteria:

- Coverage targets are met or trending upward with no regressions

## Testing Strategy

### Unit Tests

Use for:

- helpers
- serializers
- file transforms
- event construction
- archive parsing

### Integration Tests

Use for:

- public API entrypoints
- handler-level HTTP response behavior
- filesystem state changes
- fake subprocess orchestration

### Test Design Rules

- Prefer deterministic tests
- Avoid live network dependencies except explicit smoke tests
- Use temp directories instead of shared global state
- Keep tests near the module they validate when possible
- Prefer small focused tests over large end-to-end tests

## Engineering Constraints

- Preserve existing public APIs unless there is an explicit migration plan
- Avoid changing route shapes while doing structural refactors
- Avoid mixing structural refactors with unrelated feature work
- Keep changes small enough to review safely
- Do not introduce unnecessary abstractions

## Code Review Rules For This Work

- Every refactor PR should preserve behavior first
- Review should prioritize module boundaries and dependency direction
- New helpers should justify their existence through reuse or isolation value
- Large moves should be split from behavioral changes where possible

## Acceptance Criteria

This improvement project is complete only when all of the following are true:

1. Oversized backend files have been split into domain-oriented modules
2. Backend-targeted lint and test commands pass consistently
3. Coverage targets are met or a documented gap list remains with clear rationale
4. Public API behavior remains stable
5. Backend modules are easier to navigate and review

## Recommended Execution Order

To minimize risk, execute in this order:

1. Keep backend quality gates green
2. Split `dm-server` handlers
3. Split `dm-core` API facade
4. Split `dm-core` install internals
5. Split `dm-core` event internals
6. Raise coverage in low-coverage critical modules
7. Clean up workspace-wide non-backend quality blockers

## Risks And Mitigations

### Risk: Refactor Breaks Public Behavior

Mitigation:

- Preserve public signatures
- Re-export from facade modules
- Require tests before and after moves

### Risk: File Moves Create Noisy Diffs

Mitigation:

- Separate move-only commits from behavioral changes
- Use small staged refactors

### Risk: Coverage Goes Down During Refactor

Mitigation:

- Capture baseline before each phase
- Add tests before moving fragile code when practical

### Risk: Over-Abstracting The Codebase

Mitigation:

- Prefer simple module extraction over introducing traits or service layers prematurely
- Add abstraction only when multiple concrete users exist

## Deliverables

The expected deliverables for this project are:

- Refactored backend module layout
- Expanded backend automated tests
- Updated coverage baseline
- Stable backend quality gates
- Follow-up list for remaining non-backend workspace issues

## Immediate Next Step

The best immediate next step is:

1. Split `crates/dm-server/src/handlers.rs` into domain modules while preserving all routes and existing tests.
