# dm-test-media-capture

Builtin DM test node for panel-oriented screenshot capture on macOS.

## DM config

```yaml
- id: screen
  node: dm-test-media-capture
  inputs:
    trigger: some-node/output
  config:
    mode: screenshot
    output_format: png
  outputs:
    - image
    - video
    - meta
```

`image` emits PNG bytes with `content_type=image/png`.

`video` emits a short screen recording as `video/mp4` or `video/quicktime`.

`meta` emits a small JSON string describing the capture.

In `mode: screenshot`:

- if no input is connected, the node captures once on startup and exits
- if an input is connected, the node captures once for each incoming input event

Supported modes:

- `screenshot`
- `repeat_screenshot`
- `record_clip`

Example trigger-based video clip:

```yaml
- id: screen_video
  node: dm-test-media-capture
  inputs:
    trigger: some-node/output
  config:
    mode: record_clip
    clip_duration_sec: 5
    output_format: mp4
  outputs:
    - video
    - meta
```
