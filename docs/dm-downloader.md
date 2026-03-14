# dm-downloader

> Status: Implemented (TEP S=3, Δ=8%, CRYSTALIZED)

Model weight downloader node with Panel UI integration.

## Overview

A utility node that manages large file downloads (model weights, datasets, etc.) with hash verification, optional extraction, and full Panel UI lifecycle control via `bind`.

## Config

```yaml
- id: dm-downloader
  node: dm-downloader
  inputs:
    download: panel/download
  outputs:
    - path
    - ui
  config:
    url: "https://example.com/qwen-7b.tar.gz"
    hash: "sha256:abc123def..."
    extract: true
    dest: "models/qwen-7b"
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `url` | string | ✅ | — | Download URL |
| `hash` | string | ❌ | `""` | Verification hash (`algorithm:hex`), empty = skip verify |
| `extract` | bool | ❌ | `false` | Auto-extract after download (tar.gz, tar.bz2, tar.xz, zip) |
| `dest` | string | ❌ | `""` | Destination path (relative to data dir, or absolute). Empty = derive from URL |

## Default Download Directory

Downloads persist in platform data directory (not cache — survives OS cleanup):

| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/dm/downloads/` |
| Linux | `~/.local/share/dm/downloads/` |
| Windows | `%LOCALAPPDATA%/dm/data/downloads/` |

Override with `DM_DOWNLOAD_DIR` environment variable.

## Ports

| Port | Direction | Description |
|------|-----------|-------------|
| `download` | input | Trigger download (any event on this port) |
| `path` | output | Verified file/directory path, emitted once on Ready |
| `ui` | output | Widget state for Panel `bind` |

## Lifecycle

```
       Start
         │
    ┌────▼────┐
    │ Checking │  check dest + hash
    └────┬────┘
         │
    ┌────┼──────────────┐
    │    │               │
    │    ▼ match         ▼ no file / hash mismatch
    │  Ready          Waiting
    │  (emit path)    (button enabled)
    │                    │
    │               click│
    │               ┌────▼──────┐
    │               │Downloading│
    │               │(progress) │
    │               └────┬──────┘
    │                    │
    │               ┌────▼────┐
    │               │Verifying│  (skip if no hash)
    │               └────┬────┘
    │                    │
    │            ┌───────┼────────┐
    │            │ pass           │ fail
    │            │                ▼
    │    extract?│             Failed
    │   ┌───────┼──────┐    (retry btn)
    │   │ yes   │ no   │
    │   ▼       ▼      │
    │ Extracting Ready  │
    │   │    (emit path)│
    │   ▼               │
    │ Ready             │
    │ (emit path)       │
    │                   │
    └───────────────────┘
```

Key design decisions (TEP-validated):
- **Verifying always before Extracting** — verify download integrity first
- **Hash optional** — empty hash skips verify
- **Atomic download** — writes to `{dest}.dm-tmp`, renames on completion
- **Progress fallback** — `progress=-1` when `Content-Length` absent (indeterminate)

## UI Output (`bind`)

Each lifecycle state emits a different widget override:

```python
# Checking
{"loading": True, "label": "Checking...", "disabled": True}

# Waiting for user (no file)
{"loading": False, "label": "Download (2.3 GB)", "disabled": False}

# Waiting for user (hash mismatch)
{"loading": False, "label": "Re-download (hash mismatch)", "disabled": False}

# Downloading (with Content-Length)
{"loading": True, "progress": 0.6, "label": "Downloading 60%", "disabled": True}

# Downloading (no Content-Length)
{"loading": True, "progress": -1, "label": "Downloading 12.5 MB", "disabled": True}

# Verifying
{"loading": True, "label": "Verifying hash...", "disabled": True}

# Extracting
{"loading": True, "label": "Extracting...", "disabled": True}

# Ready
{"loading": False, "label": "Ready ✓", "disabled": True, "variant": "secondary"}

# Failed
{"loading": False, "label": "Retry Download", "disabled": False, "variant": "destructive"}
```

## Panel YAML

```yaml
panel:
  inputs:
    dl_ui: dm-downloader/ui
  outputs:
    - download
  widgets:
    download:
      x-widget:
        type: button
        label: "Model"
        bind: dl_ui
        span: 6
```

## Multi-model Example

Multiple instances for different model weights:

```yaml
- id: dl-asr
  node: dm-downloader
  inputs:
    download: panel/dl_asr
  outputs: [path, ui]
  config:
    url: "https://huggingface.co/distil-whisper/model.bin"
    hash: "sha256:..."
    dest: "models/whisper"

- id: dl-llm
  node: dm-downloader
  inputs:
    download: panel/dl_llm
  outputs: [path, ui]
  config:
    url: "https://modelscope.cn/qwen/qwen-7b.tar.gz"
    hash: "sha256:..."
    extract: true
    dest: "models/qwen-7b"
```
