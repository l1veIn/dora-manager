# Backend Code Health Standard

## Purpose

This document defines a unified, repeatable standard for evaluating backend code health in this repository.

It is intended to answer three questions consistently:
- Is the backend currently safe to change?
- Which module is the main source of risk?
- What must be fixed before a module is considered healthy?

The standard applies primarily to:
- `crates/dm-core`
- `crates/dm-server`

It may also be used for backend-adjacent CLI paths when they exercise backend behavior.

## Evaluation Dimensions

Every backend health review should score the codebase across five dimensions.

### 1. Build And Lint Health

This is the minimum engineering baseline.

Required checks:
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Interpretation:
- Pass: the codebase is structurally consistent and free of known lint regressions under the current rule set.
- Fail: the codebase is not healthy, regardless of feature completeness.

Rule:
- If strict workspace `clippy` fails, the overall backend health cannot be rated above `C`.

### 2. Test Health

Tests are the primary regression signal.

Required checks:
- `cargo test --workspace`
- For focused investigations, also run crate-specific suites such as:
  - `cargo test -p dm-core`
  - `cargo test -p dm-server`

Interpretation:
- Pass: the repository’s current intended behavior is executable and verified.
- Fail: either implementation and tests have drifted, or a regression exists.

Rule:
- If workspace tests fail, the overall backend health cannot be rated above `C+`.
- If a crate compiles but its tests fail, that crate is at most `C` until fixed.

### 3. Coverage Quality

Coverage is not the only quality signal, but it is the clearest indicator of unprotected behavior.

Primary command:
- `cargo llvm-cov -p dm-core --summary-only`
- `cargo llvm-cov -p dm-server --summary-only`

Use line coverage as the default decision metric.

Coverage must be interpreted at two levels:
- Crate-level total line coverage
- High-risk module line coverage

High-risk modules include any code that:
- performs filesystem mutations
- shells out to external commands
- downloads from network sources
- persists state
- transforms user data into runtime behavior

Examples in this repository:
- `dm-core`:
  - `src/node/*`
  - `src/install/*`
  - `src/events/*`
  - `src/dataflow.rs`
- `dm-server`:
  - `src/handlers/*`

Rule:
- High total coverage does not compensate for zero coverage in a risky module.
- A high-risk module below `20%` line coverage is a red flag even if crate totals look acceptable.

### 4. Structural Maintainability

This measures whether the code can keep evolving without concentrated breakage.

Key signals:
- oversized files
- mixed responsibilities in one module
- duplicated tests covering the same behavior in multiple places
- public APIs tightly coupled to implementation details
- difficult-to-isolate side effects

File size guidance:
- `< 150` lines: ideal
- `150-300` lines: normal
- `300-500` lines: review for split candidates
- `> 500` lines: should usually be split unless there is a strong reason not to

Examples of healthy structure:
- facade module with focused submodules
- I/O code separated from data models
- handlers grouped by domain
- tests colocated near the implementation they protect

Rule:
- A backend with several critical files above `500` lines should be capped at `B-` until split or strongly justified.

### 5. Risk Concentration

This measures how much important behavior depends on code paths that are hard to validate.

High-risk traits:
- external process invocation (`Command`)
- network I/O
- sparse checkout / shell-based source retrieval
- runtime path resolution
- mutation of persistent state used by other features
- fallback logic that silently changes behavior

Healthy code reduces risk concentration by:
- making error paths explicit
- failing early before network work when local preconditions already fail
- separating pure logic from side effects
- adding deterministic tests for local branches even when network branches remain untested

Rule:
- If critical behavior is primarily exercised through external systems and has no deterministic local tests, that module is at most `C+`.

## Grade Scale

Use these grades for both crate-level and module-level health.

### A

Excellent.

Conditions:
- strict lint passes
- tests pass
- crate-level coverage is strong
- high-risk modules have meaningful coverage
- structure is modular and easy to reason about
- no obvious regression hotspots

Typical threshold:
- total line coverage >= `75%`
- high-risk modules generally >= `60%`

### B

Healthy enough for normal iteration.

Conditions:
- lint passes
- tests pass
- coverage is adequate, with only a few weak spots
- structure is mostly modular
- known risks exist but are bounded and visible

Typical threshold:
- total line coverage `55-74%`
- high-risk modules mostly >= `35%`
- no critical module at `0%`

### C

Functional but fragile.

Conditions:
- some quality gates fail, or
- coverage is uneven with major blind spots, or
- key modules are oversized and tightly coupled

Typical signals:
- workspace lint or tests failing
- core modules below `20%` coverage
- repeated breakage from model changes not reflected in tests

### D

Unsafe for normal feature work.

Conditions:
- multiple quality gates fail
- regressions are likely and poorly detected
- code structure obscures ownership and change impact

Typical signals:
- broad test failure
- severe drift between implementation and tests
- critical paths largely untested and hard to isolate

## Repository Baselines

These are the default minimum targets for this repository.

### Required Green Gates

Before backend work is considered healthy:
- `cargo clippy --workspace --all-targets -- -D warnings` must pass
- `cargo test --workspace` must pass

### Coverage Baselines

Current minimum expectations:
- `dm-core` total line coverage >= `60%`
- `dm-server` total line coverage >= `50%`

High-risk module expectations:
- `dm-core/src/node/*` >= `35%` line coverage per high-risk submodule
- `dm-core/src/install/*` >= `35%` line coverage per high-risk submodule
- `dm-core/src/events/*` >= `50%` for persistence-heavy paths
- `dm-server/src/handlers/*` >= `50%` line coverage for actively used domains

Stretch targets:
- `dm-core` total >= `70%`
- `dm-server` total >= `60%`
- all high-risk submodules >= `50%`

## Standard Review Process

Every backend health review should follow this sequence.

### Step 1. Check Recent Change Context

Inspect recent commits affecting backend domains.

Recommended commands:
- `git log --oneline --decorate -10`
- `git show --stat <commit>`

Goal:
- identify whether recent risk is concentrated in `node`, `install`, `events`, `dataflow`, or `handlers`

### Step 2. Run Baseline Gates

Run:
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

If one of these fails:
- record the exact file and failure class
- downgrade the health score immediately
- do not treat coverage as the primary concern until gates are restored

### Step 3. Measure Coverage

Run:
- `cargo llvm-cov -p dm-core --summary-only`
- `cargo llvm-cov -p dm-server --summary-only`

Then inspect module-level results, especially for recently changed modules.

When coverage artifacts look stale after a file move or split, run:
- `cargo llvm-cov clean --workspace`

### Step 4. Inspect Structural Risk

Review for:
- oversized files
- mixed domains inside one file
- duplicated tests
- side effects embedded directly in top-level business logic

Use `wc -l` and targeted source inspection.

### Step 5. Produce Findings In Priority Order

Findings should be ordered by severity:
1. broken quality gates
2. broken tests / drift
3. untested risky behavior
4. structural maintainability problems
5. secondary style concerns

## Standard Finding Categories

When writing reviews, classify findings using these labels.

- `Gate Failure`
  - lint or tests fail; must be fixed first
- `Coverage Gap`
  - risky behavior lacks regression protection
- `Structural Risk`
  - file or module is too large or too coupled
- `Behavioral Risk`
  - logic likely does the wrong thing under realistic inputs
- `Maintenance Drift`
  - models or APIs changed but tests/helpers did not follow

## Example: How To Judge A Module

### Example: `node` Module

The `node` domain should be reviewed against these questions:
- Can local preconditions fail before network or process work starts?
- Are `download`, `install`, `create`, `config`, and `status` tested independently?
- Are `pip`, `uv`, and `cargo` build paths parsed correctly?
- Are network-dependent paths isolated from deterministic local logic?
- Is shelling out to `git`, `python`, `uv`, or `cargo` wrapped in testable logic boundaries?

A `node` module is not considered healthy if:
- model changes break tests and nobody noticed until clippy/test runs
- install/download code is packed into one large file
- only success paths are tested while command/network failures are untested

## Decision Rules For Future Reviews

Use these rules to avoid inconsistent scoring.

- Never give `B` or above if workspace lint is red.
- Never give `B+` or above if workspace tests are red.
- Never give `A` if a major high-risk module is below `50%` line coverage.
- Never give `A` if critical modules remain oversized and mixed-responsibility.
- Prefer a lower score with explicit reasons over an optimistic score with hand-waving.

## Recommended Reporting Template

Use this template for future backend health summaries.

### Summary
- Overall backend grade: `<grade>`
- `dm-core`: `<grade>`
- `dm-server`: `<grade>`
- Focus module (if applicable): `<grade>`

### Gate Status
- `cargo clippy --workspace --all-targets -- -D warnings`: `pass/fail`
- `cargo test --workspace`: `pass/fail`

### Coverage
- `dm-core` line: `<value>`
- `dm-server` line: `<value>`
- key modules:
  - `<module>`: `<value>`

### Findings
1. `<highest-severity issue>`
2. `<next issue>`
3. `<next issue>`

### Next Actions
1. `<first remediation>`
2. `<second remediation>`
3. `<third remediation>`

## Ownership And Maintenance

This document should be updated when any of the following changes:
- lint policy changes
- test strategy changes
- coverage thresholds are intentionally raised
- backend architecture changes enough that “high-risk modules” need to be redefined

Thresholds should move upward gradually, not downward by convenience.
