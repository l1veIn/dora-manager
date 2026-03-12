# dm-microphone

Microphone input node with runtime device selection and enable/disable control.

## Features

- **Device Discovery**: Publishes available input devices to `devices` output as JSON on startup
- **Runtime Switching**: Accepts `device_id` input to switch microphone without restarting
- **Enable/Disable**: Accepts `enabled` input to start/stop recording (default: off)
- **Continuous Streaming**: Outputs Float32 PCM audio chunks with `sample_rate` metadata

## Usage

```yaml
- id: dm-microphone
  node: dm-microphone
  inputs:
    tick: dora/timer/millis/2000
    device_id: panel/device_id
    enabled: panel/mic_enabled
  outputs:
    - audio
    - devices

- id: panel
  inputs:
    mic_devices: dm-microphone/devices
  config_schema:
    device_id:
      x-widget:
        type: select
        label: "Microphone"
        bind: mic_devices
        valueKey: id
        labelKey: name
    mic_enabled:
      default: false
      x-widget:
        type: switch
        label: "Microphone"
        switchLabel: "Enable Recording"
```

## Ports

| Port | Direction | Description |
|------|-----------|-------------|
| `audio` | output | Float32 PCM audio stream |
| `devices` | output | JSON array of available input devices |
| `enabled` | input | Start/stop recording (`true`/`false`) |
| `device_id` | input | Set active microphone by device ID |
| `tick` | input | Timer heartbeat (keep-alive) |

## Config

| Key | Env | Default | Description |
|-----|-----|---------|-------------|
| `sample_rate` | `SAMPLE_RATE` | `16000` | Audio sample rate (Hz) |
| `max_duration` | `MAX_DURATION` | `0.1` | Buffer duration before sending chunk (s) |
