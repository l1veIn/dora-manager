# dm-save

`dm-save` is a storage-family node. It writes binary payloads to disk under the current run's `out/` directory and emits the written file path for downstream nodes.

## Ports

- `data` input: binary payload to persist
- `path` output: absolute path of the file just written

## Config

- `dir`: output directory relative to `runs/:id/out/`
- `naming`: filename template, default `{timestamp}_{seq}`
- `extension`: explicit extension or `auto`
- `max_files`: optional max retained file count
- `max_total_size`: optional max retained size like `500 MB`
- `max_age`: optional max retained age like `24h`
- `overwrite_latest`: overwrite a stable file instead of appending new files

## Notes

- The node expects `DM_RUN_OUT_DIR` to be injected by Dora Manager.
- Successful writes print a `[DM-IO] SAVE ...` log line.
