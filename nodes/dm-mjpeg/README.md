# dm-mjpeg

Rust builtin node that exposes a Dora frame stream as MJPEG over HTTP for lightweight browser preview.

## Ports

- `frame` input: `UInt8` payload containing JPEG or raw pixels as declared in metadata

## HTTP endpoints

- `/stream`
- `/snapshot.jpg`
- `/healthz`

## Install

```bash
dm node install dm-mjpeg
```
