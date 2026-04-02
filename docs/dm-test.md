# dm test — Node Debug Command

> Status: Design (TEP-validated, S=3, Δ=7%)

One-command interactive smoke test for any dm-managed node.

## Motivation

Testing a node currently requires: write YAML → `dm up` → `dm start` → check logs → `dm down`. Too heavy for iterative development. `dm test` reduces this to a single command.

## Usage

```bash
# Interactive mode (human)
dm test dm-downloader --config url=https://example.com/model.bin

# Non-interactive mode (AI Agent / CI)
dm test dm-downloader --config url=https://... --auto-trigger --timeout 10
```

## Architecture

```
dm test (Rust CLI subcommand)
  │
  ├── 1. Read dm.json → extract ports, config_schema
  ├── 2. Generate test dataflow YAML (in-memory or /tmp)
  ├── 3. ensure_runtime_up()
  ├── 4. dm start → stream logs + harness output
  ├── 5. Ctrl+C or --timeout → dora stop
  └── 6. auto_down_if_idle()
```

### Test Dataflow (auto-generated)

```yaml
nodes:
  - id: sut
    node: <target-node>
    inputs:
      <input_port>: harness/<input_port>   # from harness
      tick: dora/timer/millis/2000          # auto-wired
    outputs: [<all output ports>]
    config: <merged from dm.json defaults + --config overrides>

  - id: harness
    node: dm-test-harness                  # pre-builtin, like dm-panel
    inputs:
      <output_port>: sut/<output_port>     # observe all SUT outputs
    outputs:
      - <input_port>                       # feed SUT inputs via stdin
```

### : Pre-builtin Node

Like `dm-panel`, the harness is compiled into the `dm` binary — no Python, no separate install:

```rust
// RESERVED_NODE_IDS: &[&str] = &["dm-panel", ""];

// Transpiler generates:
// path: dm
// args: ["test", "harness-serve", "--run-id", "{RUN_ID}"]
```

Implementation in `crates/dm-cli/src/test_harness.rs`, reusing `panel.rs` utilities (`arrow_to_bytes`, `extract_type_hint`, `send_json_command`).

**Dual-thread architecture** (same pattern as `panel_serve`):

| Thread | Role |
|--------|------|
| Event reader | `events.recv()` → pretty-print SUT outputs to stderr |
| Main loop | Poll stdin (interactive) or auto-trigger (non-interactive) → `send_output` |

## Input Modes

### Interactive (default)

User types commands in stdin:

```
> download                         # send "download" to port "download"
> control flush                    # send "flush" to port "control"
> @image /path/to/test.jpg         # send file bytes to port "image"
> @quit                            # exit
```

### Non-interactive (`--auto-trigger`)

Auto-sends one empty event to each input port on startup, then observes outputs until `--timeout` or SUT exits.

```bash
dm test dm-downloader --auto-trigger --timeout 15 --config url=https://...
```

Agent/CI-friendly: deterministic start → observe → exit.

## Output Format

```
🧪 Testing: dm-downloader v0.1.0
   Ports: download(in) tick(in) → path(out) ui(out)
────────────────────────────────────────
[LOG] [dm-downloader] url=https://...
[LOG] [dm-downloader] waiting for download trigger...
> download
[OUT:ui] {"loading": true, "progress": 0.5, "label": "Downloading 50%"}
[OUT:path] /Users/.../dm/downloads/readme.md
[OUT:image] <4096 bytes image/jpeg>
```

| Prefix | Source |
|--------|--------|
| `[LOG]` | Node stderr |
| `[OUT:<port>]` | dora output event (harness receives) |
| `>` | User stdin command |

Binary outputs show summary. With `--save-outputs /dir/`, also save to disk.

## Lifecycle Management

```rust
// Start
ensure_runtime_up(home, verbose).await?;   // reuse if already running
let run = start_test_dataflow(home, yaml).await?;

// Run
stream_logs_and_harness(run).await;         // until Ctrl+C or --timeout

// Cleanup
dora_stop(run.dora_uuid).await?;
auto_down_if_idle(home, false).await;       // only shuts down if nothing else running
```

No manual tracking of "did I start dora?" — `auto_down_if_idle` handles it.

## YAML Generation Rules

From `dm.json`:

| SUT Port | Wiring |
|----------|--------|
| input (tick) | `dora/timer/millis/2000` |
| input (other) | `harness/<port>` |
| output (any) | harness subscribes: `harness/<port>: sut/<port>` |

Config: `dm.json.config_schema` defaults → `--config` CLI overrides → `env:` block.

## CLI Options

```
dm test <node-id> [OPTIONS]

Options:
  --config <KEY=VALUE>     Override config (repeatable)
  --config-file <PATH>     Load config from JSON file
  --auto-trigger           Send one event to all inputs on startup
  --timeout <SECONDS>      Auto-exit after N seconds (default: 0 = manual)
  --save-outputs <DIR>     Save binary outputs to directory
```

## Scope

| ✅ In scope | ❌ Out of scope |
|-------------|----------------|
| Single node smoke test | Multi-node integration test |
| Manual trigger + observe | Automated assertions |
| dm-managed nodes (with dm.json) | External dora nodes |
| Interactive + non-interactive | Unit test framework |

## Crate Changes

| File | Change |
|------|--------|
| `crates/dm-cli/src/cmd/test.rs` | [NEW] CLI subcommand: read dm.json, generate YAML, orchestrate lifecycle |
| `crates/dm-cli/src/test_harness.rs` | [NEW] Harness node: stdin reader + output printer (reuse `panel.rs` utils) |
| `crates/dm-core/src/dataflow/transpile/passes.rs` | [MODIFY] Add `"dm-test-harness"` to `RESERVED_NODE_IDS` |
