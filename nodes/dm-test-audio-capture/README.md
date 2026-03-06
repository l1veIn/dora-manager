# dm-test-audio-capture

Builtin DM test node for fixed-duration microphone capture.

## DM config

```yaml
- id: mic
  node: dm-test-audio-capture
  config:
    mode: once
    duration_sec: 3
    sample_rate: 16000
    channels: 1
  outputs:
    - audio
    - audio_stream
    - meta
```

`audio` emits WAV bytes with `content_type=audio/wav`.
`audio_stream` emits normalized float audio samples for downstream processors such as `dora-vad`.

`meta` emits a small JSON string describing the capture.
