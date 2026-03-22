# Custom Panel UI

Allow dataflow authors to provide custom HTML interfaces for their Panel.

## Overview

The current Panel supports widget-based controls via YAML `config_schema`. For complex interaction scenarios (charts, canvas, custom layouts), users can provide a self-contained HTML page that communicates with Panel via a JavaScript SDK.

## Architecture

```
┌─────────────────────────────────┐
│ Panel (Host)                    │
│  ┌───────────────────────────┐  │
│  │ iframe: panel.html        │  │  ← Custom UI (user-provided)
│  │  - Any HTML/JS/CSS        │  │
│  │  - Communicates via SDK   │  │
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │ Default Controls          │  │  ← YAML widgets (coexist)
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

## File Convention

```
dataflows/qwen-dev/
├── flow.yml         # Dataflow definition
├── flow.json        # Instance metadata
└── panel.html       # Custom panel UI (optional)
```

## Loading Logic

```
Panel loads →
  panel.html exists? → iframe embed + postMessage bridge
  otherwise         → render config_schema widgets as usual
```

Both modes can coexist: custom HTML handles complex UI, default widgets handle basic controls.

## SDK (dm-panel-sdk)

A lightweight JS SDK (~50 lines) injected into the iframe:

```html
<script src="/api/panel-sdk.js"></script>
<script>
  const panel = new DmPanel();

  // Receive node outputs
  panel.on("asset", (inputId, data) => {
    if (inputId === "progress") {
      document.getElementById("bar").style.width = data.progress * 100 + "%";
    }
  });

  // Send commands to nodes
  document.getElementById("btn").onclick = () => {
    panel.send("download", "start");
  };

  // Receive all latest assets on connect
  panel.on("connect", (assets) => {
    console.log("Initial state:", assets);
  });
</script>
```

## SDK API

```typescript
class DmPanel {
  // Events
  on(event: "asset", callback: (inputId: string, data: any) => void): void;
  on(event: "connect", callback: (assets: Record<string, any>) => void): void;
  on(event: "disconnect", callback: () => void): void;

  // Send command (equivalent to widget sendWidget)
  send(outputId: string, value: string): Promise<void>;

  // Query current state
  getAsset(inputId: string): any | null;
}
```

## Communication Bridge

Host (Panel) ↔ iframe via `window.postMessage`:

```
iframe → host:  { type: "dm:command", outputId, value }
host → iframe:  { type: "dm:asset", inputId, data, contentType, timestamp }
host → iframe:  { type: "dm:init", assets: {...} }
```

## Benefits

| Aspect | YAML Widgets | Custom HTML |
|--------|-------------|-------------|
| Effort | Zero code | Write HTML/JS |
| Flexibility | Limited to built-in types | Unlimited |
| Use case | Simple controls | Charts, canvas, dashboards |
| Data channel | Same | Same |
| Isolation | N/A | iframe sandbox |

## Implementation Estimate

- Backend: Serve `panel.html` from dataflow directory (~10 lines)
- Frontend: iframe container + postMessage bridge (~100 lines)
- SDK: `dm-panel-sdk.js` (~50 lines)
- Total: ~160 lines of new code
