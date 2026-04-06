# dm-screen-capture

`dm-screen-capture` captures the local screen and emits encoded image frames
into the Dora graph. It does not talk to the DM media backend directly.

Typical composition:

```yaml
- id: screen-live
  node: dm-screen-capture
  outputs:
    - frame
  config:
    mode: repeat
    interval_sec: 1

- id: publish-live
  node: dm-stream-publish
  inputs:
    frame: screen-live/frame
```

Supported modes:

- `once`: capture one frame on startup and exit
- `repeat`: capture frames on a fixed interval
- `triggered`: capture a frame for each input event on `trigger`
