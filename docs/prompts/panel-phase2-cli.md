# Phase 2: dm-cli panel subcommand + transpiler

## Context

Read `docs/dm-panel-design.md` for architecture.
Phase 1 (dm-core panel module) is already implemented.

## Task

Add `dm panel` subcommand to dm-cli and update the transpiler.

### 1. CLI subcommand

In `crates/dm-cli/src/main.rs`:

Add to the `Commands` enum:

```rust
/// Panel: real-time visualization, control, and recording
#[command(subcommand)]
Panel(PanelCommands),
```

Add new enum:

```rust
#[derive(Subcommand)]
enum PanelCommands {
    /// [internal] Run as dora node (called by transpiler)
    Serve {
        /// Run ID (UUID)
        #[arg(long)]
        run_id: String,
    },
    /// Send a control command to the active panel
    Send {
        /// Output ID (e.g. "speed", "direction")
        output_id: String,
        /// Value (JSON format)
        value: String,
        /// Specific run ID (auto-discovered if omitted)
        #[arg(long)]
        run: Option<String>,
    },
}
```

### 2. `dm panel serve` implementation

This is the "dm as node" core logic. Reference `dora-hub/node-hub/terminal-print/src/main.rs`
for the DoraNode API pattern.

```rust
fn panel_serve(home: &Path, run_id: &str) -> Result<()> {
    let store = PanelStore::open(home, run_id)?;
    let (node, mut events) = DoraNode::init_from_node_id("dm-panel".into())?;
    let mut last_cmd_seq = 0i64;

    // Thread: handle dora inputs → write assets
    let store2 = store.clone();  // PanelStore needs Clone (Arc<Mutex<Connection>>)
    std::thread::spawn(move || {
        while let Some(event) = events.recv() {
            match event {
                Event::Input { id, metadata, data } => {
                    let type_hint = /* extract from metadata or infer from data.data_type() */;
                    if let Err(e) = store2.write_asset(&id.to_string(), &type_hint, /* bytes */) {
                        eprintln!("Panel write error: {e}");
                    }
                }
                Event::Stop => break,
                _ => {}
            }
        }
    });

    // Main: poll commands → inject outputs
    loop {
        for cmd in store.poll_commands(&mut last_cmd_seq)? {
            let arrow_data = /* convert cmd.value JSON to Arrow */;
            node.send_output(cmd.output_id.into(), Default::default(), arrow_data)?;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
```

### 3. `dm panel send` implementation

```rust
fn panel_send(home: &Path, output_id: &str, value: &str, run: Option<String>) -> Result<()> {
    let run_id = match run {
        Some(id) => id,
        None => {
            // Auto-discover: scan ~/.dm/panel/*/ for recently modified index.db
            let sessions = PanelStore::list_sessions(home)?;
            let active: Vec<_> = sessions.iter()
                .filter(|s| /* last_modified within last 5 minutes */)
                .collect();
            match active.len() {
                0 => bail!("No active panel session found"),
                1 => active[0].run_id.clone(),
                _ => {
                    // Print list, ask user or take most recent
                    active[0].run_id.clone()
                }
            }
        }
    };
    let store = PanelStore::open(home, &run_id)?;
    store.write_command(output_id, value)?;
    println!("✅ Sent: {} = {}", output_id, value);
    Ok(())
}
```

### 4. Transpiler change

In `crates/dm-core/src/dataflow.rs`, find the transpiler logic that converts
`node: <id>` to `path: /absolute/path`. Add handling for `dm-panel`:

```rust
// When node == "dm-panel":
// - Set path to the dm binary itself (std::env::current_exe() or "dm")
// - Set args to ["panel", "serve", "--run-id", "{RUN_ID}"]
// - RUN_ID is generated at transpile time or passed from dm start
```

### Dependencies

Add `dora-node-api` to dm-cli's Cargo.toml:

```toml
[dependencies]
dora-node-api = { workspace = true }
```

Make sure `dora-node-api` is in workspace `Cargo.toml` dependencies.
Reference `dora-hub/node-hub/terminal-print/Cargo.toml` for the version.

### Verification

```bash
# Build check
cargo build -p dm-cli

# Test serve (needs dora running, skip for unit test)
# Test send
cargo test -p dm-cli panel  # if tests added

# Manual test:
# Terminal 1: dm start my-dataflow.yml (with dm-panel in YAML)
# Terminal 2: dm panel send speed 0.5
```
