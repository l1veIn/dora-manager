# dm-downloader

Model weight downloader node with hash verification, extraction, and Panel UI lifecycle.

## Features

- **Hash verification** — `sha256:hex` format, optional
- **Auto-extraction** — tar.gz, tar.bz2, tar.xz, zip
- **Panel UI** — Real-time status via `bind` (progress, loading, disabled)
- **Atomic download** — Downloads to `.dm-tmp`, renames on success
- **Persistent storage** — Downloads to platform data directory (not cache)
- **Retry** — Failed downloads show retry button

## Config

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `url` | string | ✅ | — | Download URL |
| `hash` | string | ❌ | `""` | Verification hash (`algorithm:hex`) |
| `extract` | bool | ❌ | `false` | Auto-extract after download |
| `dest` | string | ❌ | `""` | Destination path (relative or absolute) |

## Default Download Directory

| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/dm/downloads/` |
| Linux | `~/.local/share/dm/downloads/` |
| Windows | `%LOCALAPPDATA%/dm/data/downloads/` |

Override with `DM_DOWNLOAD_DIR` environment variable.

## Dataflow Example

```yaml
- id: dl-model
  node: dm-downloader
  inputs:
    download: panel/dl_model
  outputs: [path, ui]
  config:
    url: "https://huggingface.co/model.bin"
    hash: "sha256:abc123..."
    dest: "models/my-model"

- id: panel
  node: dm-panel
  inputs:
    dl_model_ui: dl-model/ui
  outputs:
    - dl_model
  widgets:
    dl_model:
      x-widget:
        type: button
        label: "Download Model"
        bind: dl_model_ui
        span: 6
```

## Lifecycle

```
Checking → Ready (file exists + hash matches)
Checking → Waiting (no file or hash mismatch)
Waiting → Downloading (user clicks button)
Downloading → Verifying → Extracting → Ready
Downloading → Failed → Retry → Downloading...
```
