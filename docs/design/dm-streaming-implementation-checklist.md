# DM Streaming Implementation Checklist

> Status: working checklist

## Phase 1: Server Foundation

- [x] Extend DM config with media / MediaMTX settings
- [x] Add `dm-server` media runtime module
- [x] Add MediaMTX binary resolution from config / env
- [x] Add MediaMTX auto-download and versioned cache
- [x] Add MediaMTX process lifecycle management
- [x] Add media status API
- [x] Add stream viewer API on top of message snapshots

## Phase 2: Stream Protocol

- [x] Formalize `tag = "stream"` payload handling in server code
- [x] Add stream payload normalization / validation
- [x] Add stream DTOs for API responses
- [x] Add tests for stream snapshot and viewer resolution

## Phase 3: First Stream Node

- [x] Add `dm-screen-capture` node scaffold
- [x] Add `dm-stream-publish` node scaffold
- [x] Add node config schema and README
- [x] Implement stream metadata emit
- [x] Implement ffmpeg-based screen capture publish command generation
- [x] Support macOS / Linux / Windows capture modes

## Phase 4: Web Video Panel

- [x] Add `video` panel kind and registry entry
- [x] Add run page "Add Panel" support for video panel
- [x] Add stream fetch / viewer fetch flow
- [x] Add HLS/WebRTC-capable video rendering path
- [x] Add empty / unavailable states

## Phase 5: Verification

- [x] Run `cargo test -p dm-server`
- [x] Run `npm run check`
- [x] Run Python syntax validation for new node
- [x] Document remaining limitations

## Phase 6: Media Runtime Delivery

- [x] Add `mediamtx.version` and `mediamtx.auto_download` config fields
- [x] Cache downloaded MediaMTX binaries under `<DM_HOME>/bin/mediamtx/<version>/<platform>/`
- [x] Resolve release assets per host platform
- [x] Surface download / cache source in `GET /api/media/status`
- [x] Add Settings page controls for media backend configuration
- [x] Block dataflow runs when media-capable nodes require an unavailable backend
- [x] Add frontend guidance for media-capable dataflows
- [x] Replace old direct screen-stream node with capture -> publish composition
- [ ] Add checksum verification for downloaded release assets
- [ ] Add CLI management commands for install / status / cleanup
