# System Test Dataflows Plan

This document defines a dedicated set of dataflows for validating DM's run-instance layer.

The goal is not to test business logic like `qwen-dev.yml`. The goal is to exercise:

- run lifecycle
- panel storage and queries
- logs and incremental tailing
- success / failure / stop paths
- run metadata correctness

## Design Principles

- Use deterministic nodes whenever possible.
- Avoid hardware dependencies for the main test suite.
- Keep the happy-path flow runnable in a few seconds.
- Prefer synthetic data over real microphone/camera input for the primary suite.
- Treat media capture nodes as a second-stage extension.

## Available Nodes That Fit The Main Suite

These are the most useful currently installed nodes for a stable system test:

- `pyarrow-sender`
  Sends deterministic Arrow data from `DATA`.
- `pyarrow-assert`
  Validates that an input matches expected `DATA`.
- `dora-echo`
  Passes input through unchanged.
- `dora-parquet-recorder`
  Produces durable run artifacts and logs activity.
- `dm-panel`
  Validates panel integration inside a run instance.

## Nodes To Avoid In The Main Suite

These are useful later, but not for the first stable system suite:

- `dora-microphone`
- `dora-pyaudio`
- `opencv-video-capture`
- `video-encoder`

They depend on local hardware or live media input, which makes automated verification much less stable.

## Planned Test Dataflows

### 1. `tests/dataflows/system-test-happy.yml`

Purpose:

- validate a normal run with panel enabled
- validate text / JSON / binary-like payload paths
- validate logs, panel assets, and run metadata
- validate manual stop

Suggested node roles:

- `text_sender`: `pyarrow-sender` with deterministic text payload
- `json_sender`: `pyarrow-sender` with deterministic JSON-string payload
- `bytes_sender`: `pyarrow-sender` with deterministic integer array payload
- `text_echo`: `dora-echo`
- `json_echo`: `dora-echo`
- `bytes_echo`: `dora-echo`
- `text_assert`: `pyarrow-assert`
- `json_assert`: `pyarrow-assert`
- `bytes_assert`: `pyarrow-assert`
- `recorder`: `dora-parquet-recorder`
- `panel`: `dm-panel`

Coverage:

- run starts successfully
- `run.json` has `has_panel=true`
- panel directory and `index.db` exist
- logs exist for multiple nodes
- `dm runs logs <run_id> <node> --follow` shows incremental output
- panel assets can be queried through run-scoped API
- manual `dm runs stop <run_id>` lands in `stopped`

Notes:

- JSON can be sent as a string payload first. That is enough to exercise panel text/json-like display without introducing custom nodes yet.
- "Binary-like" payload can be an integer array. It validates file storage behavior even if it is not yet a true image/audio/video asset.

### 2. `tests/dataflows/system-test-finish.yml`

Purpose:

- validate natural completion
- validate `succeeded`
- validate final log synchronization and recent-run summaries

Suggested node roles:

- one-shot `pyarrow-sender`
- `dora-echo`
- `pyarrow-assert`
- optional `dora-parquet-recorder`
- optional `dm-panel`

Coverage:

- run starts
- nodes emit expected payload
- graph exits on its own
- run final status becomes `succeeded`
- `termination_reason=completed`
- logs remain readable after completion

Constraint:

- Current generic sender nodes are one-shot and naturally fit this case.

### 3. `tests/dataflows/system-test-fail.yml`

Purpose:

- validate controlled failure
- validate `failed`
- validate failure metadata

Suggested node roles:

- `pyarrow-sender`
- `pyarrow-assert` configured with intentionally wrong expected payload
- optional `dm-panel`

Coverage:

- run starts
- one assertion node fails deterministically
- final run status becomes `failed`
- `termination_reason=node_failed`
- `failure_message` is populated
- logs capture the failure

This is the simplest reliable failure mode using currently installed nodes.

### 4. `tests/dataflows/system-test-no-panel.yml`

Purpose:

- validate the non-panel path explicitly
- validate server rejection of panel access

Suggested node roles:

- `pyarrow-sender`
- `dora-echo`
- `pyarrow-assert`

Coverage:

- run starts without panel
- `has_panel=false`
- `/api/runs/{id}/panel/*` returns a clear error
- run lifecycle still works normally

## Gaps That Need Custom Test Nodes Later

The first suite can cover run/panel/log semantics without custom nodes.

To cover media assets well, we should add purpose-built nodes later:

### A. `dm-test-media-capture`

Purpose:

- capture deterministic screenshots or short screen recordings
- emit image/video assets into panel

Potential modes:

- screenshot once
- screenshot every N seconds
- record screen for N seconds

Recommended scope:

- target macOS first
- use built-in platform capture tooling rather than OpenCV
- optimize for test determinism, not general-purpose capture UX

Suggested outputs:

- `image`: PNG bytes for screenshot mode
- `video_file`: relative file path for short recording mode
- `meta`: JSON text describing width, height, duration, and timestamp

Suggested DM config:

- `mode`: `screenshot` | `record`
- `duration_sec`: integer, only used for `record`
- `interval_ms`: integer, only used for repeated screenshots
- `output_format`: `png` | `mp4`
- `window_name`: optional capture target hint

Implementation notes:

- for screenshot mode, emit a stable single PNG file into the run panel directory
- for recording mode, prefer writing an `.mp4` file and emitting a panel file asset reference
- node should log the exact capture command it used
- node must exit cleanly on Dora stop
- node should be acceptable even if capture permissions are not granted; failure should be explicit in logs

Why not reuse `opencv-video-capture`:

- it is camera-oriented, not screen-oriented
- it has no DM config schema today
- it is less deterministic for system tests than platform screen capture

### B. `dm-test-audio-capture`

Purpose:

- record loopback or microphone input while local music is playing
- emit audio bytes or files into panel

Potential modes:

- record fixed duration
- emit a saved wav file

Recommended scope:

- target macOS first
- support microphone capture first
- leave loopback/system audio as optional follow-up because the setup is host-dependent

Suggested outputs:

- `audio_file`: relative path to `.wav`
- `audio_meta`: JSON text describing sample rate, channels, duration, and byte size
- optional `waveform_png`: preview image for panel if cheap to generate

Suggested DM config:

- `source`: `microphone` | `loopback`
- `duration_sec`: integer
- `sample_rate`: integer
- `channels`: integer
- `format`: `wav`

Implementation notes:

- write the captured `.wav` file under the current run directory
- emit the file as a panel asset instead of streaming raw PCM through the graph
- prefer deterministic fixed-duration capture
- node must flush and close the file before exit
- node must exit cleanly on Dora stop

Why not reuse `dora-pyaudio`:

- it is a generic runtime node, not a focused test node
- it has no DM config schema today
- microphone and loopback behavior can vary by host, so the test wrapper should make that explicit

## Proposed Node Metadata

Both media nodes should be first-class DM nodes with `config_schema` so that DM
dataflows can use `config:` instead of raw `env:`.

Suggested schema shape:

```json
{
  "mode": { "env": "MODE", "default": "screenshot" },
  "duration_sec": { "env": "DURATION_SEC", "default": 3 },
  "interval_ms": { "env": "INTERVAL_MS", "default": 1000 },
  "source": { "env": "SOURCE", "default": "microphone" },
  "sample_rate": { "env": "SAMPLE_RATE", "default": 16000 },
  "channels": { "env": "CHANNELS", "default": 1 }
}
```

We do not need every field on every node, but both nodes should expose
configuration through DM's transpile layer.

## Proposed Test Coverage With Media Nodes

After the first four synthetic flows are stable, add:

### `system-test-screen.yml`

- starts a run with panel
- captures one screenshot
- optionally records a short clip
- verifies image/video assets land in `panel/`
- verifies run can stop cleanly

### `system-test-audio.yml`

- starts a run with panel
- records a short wav clip while local sound is playing
- verifies audio asset lands in `panel/`
- verifies metadata is queryable
- verifies run can stop cleanly

## Recommended Implementation Order

1. Write `system-test-happy.yml`
2. Write `system-test-finish.yml`
3. Write `system-test-fail.yml`
4. Write `system-test-no-panel.yml`
5. Add a small verification checklist for each flow
6. Only then add custom media test nodes

## Verification Checklist Template

For each run we should check:

- `dm runs`
- `cat ~/.dm/runs/<run_id>/run.json`
- `find ~/.dm/runs/<run_id> -maxdepth 3 -print | sort`
- `dm runs logs <run_id>`
- `dm runs logs <run_id> <node_id>`
- panel asset query
- final run status and termination reason

## Recommendation

Proceed with YAML/dataflow authoring first.

That will lock down:

- the exact assertions we care about
- the exact data types we can already cover
- the exact gaps that justify new media capture nodes
