#!/usr/bin/env python3
"""
dm-sdk-demo: All-in-one interaction demo using the dm SDK.

Shows how a dora node can use the dm SDK to:
1. Register widgets (no bridge needed)
2. Listen for user input from the Web UI (SDK subscribe/get)
3. Send results back to the Web UI (SDK send)

No bridge involved. This node has no Arrow ports — all communication
goes through dm-server via HTTP + WebSocket.

Prerequisites:
  - dm-server running (auto-managed by dm-core)
  - SDK installed: pip install ./sdk/python/
"""

import json
import os
import signal
import sys
import time

import pyarrow as pa
from dora import Node

# Add SDK to path for development
SDK_PATH = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "sdk", "python"
)
SDK_PATH = os.path.normpath(SDK_PATH)
if os.path.isdir(SDK_PATH) and SDK_PATH not in sys.path:
    sys.path.insert(0, SDK_PATH)

import dm  # noqa: E402


RUNNING = True

# ── Widget configuration ──
WIDGET_KEY = "sdk-demo-input"

# ── Demo state ──
COUNTER = 0
CHAT_HISTORY: list[str] = []


def handle_signal(signum, frame):
    global RUNNING
    RUNNING = False


signal.signal(signal.SIGINT, handle_signal)
signal.signal(signal.SIGTERM, handle_signal)


def on_new_message(msg: dm.Message, event: dict):
    """Called when a new input event arrives from widget subscribe."""
    global COUNTER, CHAT_HISTORY
    value = event.get("value", "")
    if not value:
        return
    COUNTER += 1
    CHAT_HISTORY.append(value)
    if len(CHAT_HISTORY) > 10:
        CHAT_HISTORY.pop(0)
    reversed_str = value[::-1]
    word_count = len(value.split())
    eprint(f"[dm-sdk-demo] #{COUNTER} '{value}' -> '{reversed_str}'")

    # ── Send 1: Simple text (legacy style) ──
    msg.send("text", {"content": f"**Reversed:** {reversed_str}"}, from_="dm-sdk-demo")

    # ── Send 2: Rich embed card ──
    msg.send(
        "text",
        {
            "content": f"Embed: {value}",
            "embed": {
                "author": {"name": "SDK Demo", "icon": "🧪"},
                "title": f"Input #{COUNTER}",
                "body": value,
                "body_formatted": f"**Reversed:** {reversed_str}  \nWord count: **{word_count}**",
                "body_format": "markdown",
                "color": "blue" if word_count < 5 else "purple",
                "fields": [
                    {"name": "Original", "value": value, "inline": True},
                    {"name": "Reversed", "value": reversed_str, "inline": True},
                    {"name": "Word Count", "value": str(word_count), "inline": True},
                ],
                "footer": {"text": f"message #{COUNTER} via dm-sdk-demo"},
                "side": "left",
            },
        },
        from_="dm-sdk-demo",
    )

    # ── Send 3: Chat-style (right side, user-like) ──
    msg.send(
        "text",
        {
            "content": value,
            "embed": {
                "body": value,
                "side": "right",
                "width": "compact",
                "color": "gray",
                "timestamp": "absolute",
            },
        },
        from_="dm-sdk-demo",
    )

    # ── Send 4: Stats embed every 3 messages ──
    if COUNTER % 3 == 0:
        msg.send(
            "text",
            {
                "content": f"Stats after #{COUNTER}",
                "embed": {
                    "author": {"name": "SDK Demo Stats", "icon": "📊"},
                    "color": "green",
                    "fields": [
                        {"name": "Total Inputs", "value": str(COUNTER), "inline": True},
                        {"name": "History Size", "value": f"{len(CHAT_HISTORY)}/10", "inline": True},
                        {"name": "Last Input", "value": CHAT_HISTORY[-1] if CHAT_HISTORY else "—", "inline": False},
                    ],
                    "actions": [
                        {"label": "Reset Counter", "url": "#", "style": "button"},
                    ],
                },
            },
            from_="dm-sdk-demo",
        )


def main():
    global RUNNING

    # Initialize SDK
    msg = dm.Message()
    eprint(f"[dm-sdk-demo] starting, run_id={msg.run_id}")

    # Register widget using the new WidgetManager API
    msg.widgets.register(
        key=WIDGET_KEY,
        type="input",
        label="Text to reverse",
        config={
            "placeholder": "Type something and press Enter...",
        },
    )

    # Send a startup message
    msg.send("text", {"content": "✅ SDK Demo node started — type something to reverse"}, from_="dm-sdk-demo")

    # Keep the main thread alive (dora node convention) + subscribe
    node = Node()
    for event in msg.widgets.subscribe(WIDGET_KEY):
        if not RUNNING:
            break
        on_new_message(msg, event)

    RUNNING = False
    eprint("[dm-sdk-demo] shutting down")
    msg.send("text", {"content": "⏹️ SDK Demo stopped"}, from_="dm-sdk-demo")


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


if __name__ == "__main__":
    main()
