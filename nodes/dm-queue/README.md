# dm-queue

Rust builtin node that provides FIFO buffering, flush signaling, ring overwrite mode, and spool-to-file fallback for Dora Manager dataflows.

## Ports

- `data` input: `UInt8` or UTF-8 payload with stream metadata in dora parameters
- `control` input: UTF-8 command string (`flush`, `reset`, `stop`)
- `flushed` output: JSON summary string
- `buffering` output: JSON status string
- `error` output: JSON error string

## Install

```bash
dm node install dm-queue
```
