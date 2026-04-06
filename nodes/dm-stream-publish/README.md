# dm-stream-publish

`dm-stream-publish` accepts encoded image frames from the Dora graph, publishes
them to the DM media backend through `ffmpeg`, and emits a `stream` message so
the web workspace can render the live feed in `VideoPanel`.

Typical composition:

```yaml
- id: screen-live
  node: dm-screen-capture
  outputs:
    - frame

- id: preview-publish
  node: dm-stream-publish
  inputs:
    frame: screen-live/frame
  config:
    label: "Live Screen"
    stream_name: live
```
