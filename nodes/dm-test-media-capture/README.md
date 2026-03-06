# dm-test-media-capture

Builtin DM test node for panel-oriented screenshot capture on macOS.

## DM config

```yaml
- id: screen
  node: dm-test-media-capture
  config:
    mode: screenshot
    output_format: png
  outputs:
    - image
    - meta
```

`image` emits PNG bytes with `content_type=image/png`.

`meta` emits a small JSON string describing the capture.

Supported modes:

- `screenshot`
- `repeat_screenshot`
