# Interaction Rebuild Todo

> Goal: rebuild the removed panel capabilities as orthogonal storage-family and interaction-family nodes, without reintroducing node-specific coupling into `dm-core`.

## Phase 1: Storage Family

- [x] Inject generic run context env for managed nodes:
  `DM_RUN_ID`, `DM_RUN_OUT_DIR`, `DM_SERVER_URL`
- [x] Ensure run layout consistently exposes `runs/:id/out/` as the artifact root
- [x] Implement `dm-save`
  - [x] bytes input -> file output
  - [x] relative path config rooted at `DM_RUN_OUT_DIR`
  - [x] retention cleanup: `max_age`, `max_files`, `max_total_size`
  - [x] emit `path` output
  - [x] print `[DM-IO] SAVE ...`
- [x] Implement `dm-log`
  - [x] text/json/csv serialization
  - [x] rotation / retention / compression
  - [x] emit `path` output for downstream display nodes
  - [x] print `[DM-IO] LOG ...`
- [x] Implement `dm-recorder`
  - [x] append/aggregate audio chunks
  - [x] flush to single artifact
  - [x] emit `path` output
- [x] Add example dataflow for storage + interaction wiring
- [x] Add focused tests for transpile env injection and storage/relay path behavior

## Phase 2: Interaction Family + Relay

- [x] Define dm-server relay model for display and input sessions
  - [x] persisted per-run JSON state under `runs/:id/interaction/`
  - [x] display announcement API
  - [x] input registration API
  - [x] input event claim API
  - [x] artifact file serving API
- [x] Implement `dm-display`
  - [x] input-driven path mode
  - [x] static source polling mode
  - [x] render mode inference
  - [x] notify dm-server only, no local server
- [x] Implement `dm-input`
  - [x] widget config schema
  - [x] relay registration protocol
  - [x] polling event claim loop
  - [x] emit Arrow outputs
- [x] Rebuild web run detail UI around display/input relay
  - [x] display panes from server session state
  - [x] input widgets from `dm-input` config
  - [x] no panel-specific assumptions in run model
- [x] Add end-to-end example dataflow using:
  compute -> storage -> display
  browser -> input -> compute

## Guardrails

- [x] No node-specific fields in `dm-core` run model
- [x] No reserved node IDs in transpile
- [x] No widget extraction in `dm-core`
- [x] No node starts its own HTTP server for browser interaction
- [x] Storage, display, and input remain split into separate nodes
