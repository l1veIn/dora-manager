# Message Format Protocol: Embed Render Description

> Message format protocol v1

## Overview

Dora Manager's message system is built around five core fields: `seq`, `from`, `tag`, `payload`, and `timestamp`. The `payload` field is a free-form JSON object whose structure is determined by the sending node. To support rich formatted message rendering, we introduce an optional `payload.embed` field — a **structured render description object**.

**Core design principle:** No existing code needs to change. Messages without `embed` render exactly as before. Adding `embed` unlocks card-style rich rendering.

## Message Structure

```json
{
  "seq": 42,
  "from": "ai-assistant",
  "tag": "text",
  "payload": {
    "content": "Hello world",
    "embed": { ... }
  },
  "timestamp": 1715000000000
}
```

The optional `embed` object inside `payload` controls how the message renders in the frontend.

## Embed Field Reference

### Position & Layout

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `side` | `"left"` / `"right"` / `"center"` | `"left"` | Message bubble alignment |
| `width` | `"full"` / `"compact"` | `"compact"` | Message width |

### Sender Info

```json
"author": {
  "name": "AI Detector v2",
  "icon": "🤖",
  "url": "/nodes/detector"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `author.name` | string | yes | Display name |
| `author.icon` | string | no | Inline icon (emoji or icon name) |
| `author.url` | string | no | Click-through link |

### Title

```json
"title": "Detection Result",
"title_url": "https://..."
```

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Card title (renders as card header) |
| `title_url` | string | Clickable title link |

### Body Content (Matrix-style)

```json
"body": "Person detected at 0.95 confidence",
"body_formatted": "**Person** detected at *0.95* confidence",
"body_format": "markdown"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `body` | string | — | Plain text fallback |
| `body_formatted` | string | — | Rich text content (optional) |
| `body_format` | `"plain"` / `"markdown"` | `"plain"` | Rich text format type |

Frontend rule: `body_formatted` is preferred when present; fall back to `body`. When `body_format` is `"markdown"`, render `body_formatted` with a markdown renderer.

### Color Accent Bar

```json
"color": "green"
"color": 0x22c55e
```

| Value type | Example | Notes |
|------------|---------|-------|
| Semantic name | `"green"`, `"red"`, `"yellow"`, `"blue"`, `"purple"`, `"gray"`, `"orange"` | Predefined colors |
| Hex integer | `0x22c55e` | RGB color value |
| Hex string | `"#22c55e"` | CSS-style hex |

### Structured Fields

```json
"fields": [
  { "name": "Class", "value": "person", "inline": true },
  { "name": "Confidence", "value": "0.95", "inline": true },
  { "name": "Bounding Box", "value": "x:120 y:80 w:200 h:300" }
]
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fields[].name` | string | — | Field label |
| `fields[].value` | string | — | Field value |
| `fields[].inline` | boolean | `false` | Display inline on the same row |

Fields with `inline: true` are arranged side-by-side where space permits.

### Media Attachment

```json
"media": {
  "url": "/api/runs/{id}/artifacts/snapshot.jpg",
  "type": "image",
  "width": 640,
  "height": 480,
  "alt": "Detection result screenshot",
  "caption": "Frame #142 — person detected"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `media.url` | string | yes | Media resource URL |
| `media.type` | `"image"` / `"video"` / `"audio"` | yes | Media type |
| `media.width` | integer | no | Display width (pixels) |
| `media.height` | integer | no | Display height (pixels) |
| `media.alt` | string | no | Alternative text |
| `media.caption` | string | no | Caption below the media |

### Thumbnail

```json
"thumbnail": {
  "url": "/api/runs/{id}/artifacts/thumb.jpg",
  "width": 80,
  "height": 80
}
```

Small image displayed in the top-right corner of the card. Only `url` is required.

### Footer

```json
"footer": {
  "text": "dora-yolo v0.3.1",
  "icon": "⚡"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `footer.text` | string | yes | Footer text |
| `footer.icon` | string | no | Footer icon |

### Timestamp Display

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timestamp` | `"relative"` / `"absolute"` / `"hidden"` | `"relative"` | Time display mode |

### Action Buttons

```json
"actions": [
  { "label": "View Details", "url": "/runs/{id}/nodes/detector", "style": "link" },
  { "label": "Download", "url": "/api/artifacts/report.json", "style": "button" }
]
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `actions[].label` | string | yes | Button text |
| `actions[].url` | string | yes | Click-through URL |
| `actions[].style` | `"link"` / `"button"` | no | Display style |

### Status Indicator

```json
"status": "running",
"progress": 0.75
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | `"pending"` / `"running"` / `"success"` / `"warning"` / `"error"` | Message status |
| `progress` | float (0.0~1.0) | Progress percentage (only relevant when `status=running`) |

## Render Priority

The frontend uses whatever embed fields are present to determine render shape:

| embed contains | renders as |
|---|---|
| No embed | Falls back to traditional tag+payload rendering |
| Only `body` | Plain text message |
| `body` + `body_formatted` | Rich text (markdown-capable) |
| `color` | Color accent bar |
| `author` | Sender info header |
| `title` | Clickable title |
| `fields` | Structured field table |
| `media` | Media embed card |
| `thumbnail` | Top-right thumbnail |
| `footer` | Footer info row |
| `actions` | Action button row |
| `status` + `progress` | Status bar / progress bar |
| `side` | Bubble alignment |

When multiple fields are present, they follow this layout order: `author` > `title` > `body`/`fields` > `media` > `thumbnail` > `footer` > `actions`.

## SDK Usage

### Python SDK

```python
import dm

# Method 1: set a default embed template
msg = dm.Message()
msg.embed(
    author={"name": "AI Assistant", "icon": "🤖"},
    color="blue",
    footer={"text": "dora-yolo v0.3"},
)

# All subsequent sends carry the embed automatically
msg.send("text", {"content": "Hello"})
# → payload includes:
#   {content: "Hello", embed: {author: {name: "AI Assistant", icon: "🤖"}, color: 0x22c55e, ...}}

# Method 2: override embed per-message
msg.send("text", {"content": "Warning!"}, embed={"color": "red"})

# Method 3: chaining
msg = dm.Message().embed(author={"name": "Bot"}, color="green")

# Method 4: no embed (traditional, fully backward compatible)
msg.send("text", {"content": "hello"})  # no embed
```

### Using embed in a dm-display node

```python
import dm

msg = dm.Message()
msg.send("text", {
    "content": "Detection result",
    "embed": {
        "author": {"name": "YOLO Detector", "icon": "🔍"},
        "body_formatted": "**Person** detected at *0.95* confidence",
        "body_format": "markdown",
        "color": "green",
        "fields": [
            {"name": "Class", "value": "person", "inline": True},
            {"name": "Confidence", "value": "0.95", "inline": True},
        ],
    }
})
```

### Frontend JavaScript consumption

MessageItem.svelte checks for `entry.payload.embed`:
- Present → use the embed render pipeline (card layout)
- Absent → fall back to traditional tag+payload rendering

## Backward Compatibility

No existing code needs modification. Embed is entirely optional. All of the following messages are valid:

```python
# Legacy (no embed)
msg.send("text", {"content": "hello"})

# New (full embed)
msg.send("text", {"content": "hello", "embed": {"body": "hello"}})

# New (embed only, ignoring legacy payload fields)
msg.send("text", {"anything": "goes", "embed": {"body": "hello"}})
```

## Related Reading

- [Interaction System Architecture](22-interaction-system.md) — The overall message system architecture
- [Reactive Widgets](20-reactive-widgets.md) — Widget registration and frontend rendering
