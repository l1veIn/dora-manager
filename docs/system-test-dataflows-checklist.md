# System Test Dataflows Checklist

This checklist is for manual validation of the DM run-instance layer using the
test dataflows in `/Users/yangchen/Desktop/dora-manager/tests/dataflows`.

## Shared checks

Start a run:

```bash
dm start /Users/yangchen/Desktop/dora-manager/tests/dataflows/<flow>.yml
```

Record the returned `run_id`, then verify:

```bash
dm runs
cat ~/.dm/runs/<run_id>/run.json
find ~/.dm/runs/<run_id> -maxdepth 3 -print | sort
dm runs logs <run_id>
```

Expected during or after execution:

- `run.json` contains the expected `dataflow_name`
- `nodes_expected` matches the YAML node IDs
- `transpile.resolved_node_paths` is populated
- `out/` contains raw Dora logs
- `logs/` appears after `dm runs logs <run_id>` or status refresh

For panel-enabled flows also verify:

```bash
sqlite3 ~/.dm/runs/<run_id>/panel/index.db '.tables'
```

Expected:

- `has_panel=true` in `run.json`
- `panel/index.db` exists

For no-panel flows verify:

- `has_panel=false` in `run.json`
- `~/.dm/runs/<run_id>/panel` does not exist unless panel was accessed by mistake

## system-test-happy.yml

Command:

```bash
dm start /Users/yangchen/Desktop/dora-manager/tests/dataflows/system-test-happy.yml
```

Expected:

- run starts successfully
- text, JSON, and bytes pipelines all produce logs
- `panel/index.db` exists
- `recorder` creates parquet output under the run directory

Suggested checks:

```bash
dm runs logs <run_id> text_sender
dm runs logs <run_id> json_sender
dm runs logs <run_id> bytes_sender
dm runs logs <run_id> recorder
find ~/.dm/runs/<run_id>/panel -maxdepth 2 -print | sort
find ~/.dm/runs/<run_id> -path '*recorder*' -print | sort
```

## system-test-finish.yml

Command:

```bash
dm start /Users/yangchen/Desktop/dora-manager/tests/dataflows/system-test-finish.yml
```

Expected:

- run reaches terminal state without manual stop
- final `status` is `succeeded`
- `termination_reason` is completion-oriented rather than user stop

Suggested checks:

```bash
dm runs
cat ~/.dm/runs/<run_id>/run.json
dm runs logs <run_id>
```

## system-test-fail.yml

Command:

```bash
dm start /Users/yangchen/Desktop/dora-manager/tests/dataflows/system-test-fail.yml
```

Expected:

- run reaches terminal state with `status=failed`
- `termination_reason` is `node_failed`
- `failure_node` points to `fail_assert`
- `failure_message` contains the assertion mismatch
- `fail_assert` log contains the same assertion failure details

Suggested checks:

```bash
dm runs
cat ~/.dm/runs/<run_id>/run.json
dm runs logs <run_id> fail_assert
```

## system-test-no-panel.yml

Command:

```bash
dm start /Users/yangchen/Desktop/dora-manager/tests/dataflows/system-test-no-panel.yml
```

Expected:

- run starts successfully
- `has_panel=false`
- no `panel/` directory is created during normal execution

Suggested checks:

```bash
cat ~/.dm/runs/<run_id>/run.json
find ~/.dm/runs/<run_id> -maxdepth 2 -print | sort
```

## system-test-screen.yml

Install the builtin node first:

```bash
dm node install dm-test-media-capture
```

Command:

```bash
dm start /Users/yangchen/Desktop/dora-manager/tests/dataflows/system-test-screen.yml
```

Expected:

- run starts successfully
- `has_panel=true`
- panel stores one PNG asset and one JSON metadata asset
- node logs include the screenshot command

Suggested checks:

```bash
dm runs logs <run_id> screen
find ~/.dm/runs/<run_id>/panel -maxdepth 3 -print | sort
sqlite3 ~/.dm/runs/<run_id>/panel/index.db 'select seq,input_id,type,storage,path from assets order by seq;'
```

## system-test-audio.yml

Install the builtin node first:

```bash
dm node install dm-test-audio-capture
```

Command:

```bash
dm start /Users/yangchen/Desktop/dora-manager/tests/dataflows/system-test-audio.yml
```

Expected:

- run starts successfully
- `has_panel=true`
- panel stores one WAV asset and one JSON metadata asset
- final run status is `succeeded`

Suggested checks:

```bash
dm runs logs <run_id> microphone
find ~/.dm/runs/<run_id>/panel -maxdepth 3 -print | sort
sqlite3 ~/.dm/runs/<run_id>/panel/index.db 'select seq,input_id,type,storage,path from assets order by seq;'
```
