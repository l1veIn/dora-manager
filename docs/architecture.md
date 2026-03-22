# Dora Manager (dm): The `pip` & `ffmpeg` of dora-rs

To evolve `dm` from a simple environment manager into the central package manager (`pip/npm`) and CLI orchestrator (`ffmpeg`) for the dora-rs ecosystem, we need to extend its architecture to understand, acquire, and execute **Nodes**.

Looking at the current [Node Registry Data](https://raw.githubusercontent.com/dora-rs/dora-rs.github.io/main/src/data/nodes.json), each node contains metadata such as the `build` instruction (e.g., `pip install dora-distil-whisper`) and the dataflow YAML snippet (`inputs`, `outputs`).

Here is a proposed architectural design and workflow for this next phase.

---

## 1. Node Package Manager (`pip/npm` equivalent)

Currently, users have to manually configure Python virtual environments, install dependencies, and write YAML files. `dm` can automate this completely.

### The Node Package Standard
Thanks to your insight (e.g., `dora-pyaudio`), it's clear that **a dora-rs node is fundamentally a native language package**. 
*   **Python Nodes**: Standard `pip` packages complete with `pyproject.toml`, defining executable entry points (e.g., `[project.scripts] dora-pyaudio = "..."`).
*   **Rust Nodes**: Standard `cargo` crates compiled to standalone binaries.
*   **C/C++ Nodes**: Compiled native binaries.

Because nodes are just standard packages, `dm` doesn't need to invent a proprietary package format. Instead, `dm` acts as a **smart wrapper around existing ecosystem tools** (like `uv`/`pip` or `cargo`), layering Dora-specific metadata and sandboxing.

### CLI Commands
*   `dm search <query>`: Queries the remote `nodes.json` registry (or a local cache) to find nodes (e.g., `dm search whisper`).
*   `dm info <node-id>`: Displays the node's description, inputs, outputs, and requirements.
*   `dm add <node-id>` (or `dm install`):
    *   Downloads the node source or prebuilt binary.
    *   **Isolation & Build**: 
        *   For **Python nodes**: `dm` uses `uv` to create an isolated virtual environment (`uv venv`) and runs the package's native `pip install`.
        *   For **Rust nodes**: `dm` detects the `cargo build` instruction (e.g., `cargo build -p dora-rav1e --release`) and compiles the standalone binary, isolating the resulting executable in the node's `~/.dm/nodes/<node-id>/bin` directory.
    *   Executes the `build` instruction defined in the registry.
*   `dm list`: Lists locally installed nodes.

### Handling System Dependencies (The Interactive Helper)
Many advanced nodes (like LLMs or computer vision nodes) require system-level C-libraries (e.g., `cmake`, `libportaudio2`) before `pip` or `cargo` can compile them. `dm` will act as an **Interactive Helper**:
*   The `nodes.json` registry can define a `system_deps` field per OS (e.g., `macos: "brew install cmake"`).
*   During `dm add`, `dm` parses this and **pauses**, showing an interactive prompt:
    `The node 'llm-node' requires system dependencies: cmake. Install via Homebrew? [Y/n]`
*   If accepted, `dm` securely delegates to the OS package manager (`brew`, `apt`, `winget` for Windows), making cross-platform expansion (like to Windows) seamless without taking dangerous unprompted actions.

### Local Node Cache Structure
```text
~/.dm/
├── config.toml
├── versions/              # dora core binaries
└── nodes/                 # Installed node definitions
    ├── dora-distil-whisper/
    │   ├── venv/          # Isolated python env
    │   └── node.yml       # Cached metadata (inputs, outputs, run cmd)
    └── webcam/
        ├── bin/           # Rust/C++ binary isolate
        └── node.yml 
```

---

## 2. Dynamic Execution Engine (`ffmpeg` equivalent)

`ffmpeg` is powerful because it allows complex media processing pipelines purely via CLI flags without writing config files. `dm` can do the same for dataflows.

### Single Node Execution (`dm run`)
Run a single node for testing, automatically mapping stdin/stdout to dora inputs/outputs.
```bash
# Automatically creates a temporary dataflow.yml and runs it
$ dm run dora-distil-whisper --input audio=./test.wav --output text
```

### Linear Pipeline Synthesis (`dm pipe`)
Since the registry metadata defines the `inputs` and `outputs` for each installed node, `dm` can infer the topology and automatically stitch them together.

```bash
# Syntax idea: Use '!' or '|' to separate nodes
$ dm pipe "webcam ! object-detection ! display"
```
**How it works under the hood**:
1. `dm` parses the string into three ordered nodes.
2. It looks up `webcam` in `~/.dm/nodes/`, sees it produces `image`.
3. It looks up `object-detection`, sees it takes `image` and outputs `bbox`.
4. It dynamically generates a `dataflow.yml` doing the exact wiring.
5. It calls `dora up && dora start generated_dataflow.yml`.

---

## 3. Dataflow Scaffolding (`dm init` and Templates)

For more complex, non-linear graphs (like multiple rust and python nodes interacting), users still need YAML. `dm` can act like `npm init` or `cargo new`.

*   `dm new my_project` -> Creates a blank `dataflow.yml`.
*   `dm new my_project --template av1-encoding` -> Fetches an example dataflow from `dora-hub/examples`, automatically executing `dm add` for every node defined in the example's YAML, and generating a ready-to-run local `dataflow.yml`.
*   `dm add dora-qwen2-5-vl` -> Appends the node's YAML template directly into your local `dataflow.yml` file, automatically setting up the `path` to point to the `~/.dm/nodes/dora-qwen2-5-vl` isolated environment.

---

## 4. Required Core Crate Modifications (`dm-core`)

To realize this, `dm-core` needs three new subsystems:

1.  **Registry Client (`registry.rs`)**:
    *   Fetch `nodes.json` from the Dora Github API.
    *   Deserialize with `serde_json` into a `NodeMeta` struct.
2.  **Node Manager (`node.rs`)**:
    *   Logic for sandboxing: `uv venv` creation for Python nodes, `cargo binstall` for Rust nodes.
3.  **Dataflow Synthesizer (`graph.rs`)**:
    *   In-memory generation of dora-rs YAML configurations based purely on CLI arguments for the `dm pipe` and `dm run` features.

## Summary

By doing this, `dm` stops being just a "version manager" and becomes the **App Store and execution engine** for the Dora ecosystem, massively lowering the barrier to entry for AI/robotic pipelines.
