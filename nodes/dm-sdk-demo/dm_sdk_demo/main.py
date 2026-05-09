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


def input_value(event: dict):
    """Return the widget value from either a full SDK message or a flat event."""
    payload = event.get("payload")
    if isinstance(payload, dict) and "value" in payload:
        return payload.get("value")
    return event.get("value")


def send_reply(msg: dm.Message, tag: str, payload: dict) -> int | None:
    try:
        seq = msg.send(tag, payload, from_="dm-sdk-demo")
    except Exception as exc:
        eprint(f"[dm-sdk-demo] failed to send {tag!r} reply: {exc!r}")
        return None
    eprint(f"[dm-sdk-demo] sent {tag!r} reply, seq={seq}")
    return seq


def on_new_message(msg: dm.Message, event: dict):
    """Called when a new input event arrives from widget subscribe."""
    global COUNTER, CHAT_HISTORY
    value = input_value(event)
    if value is None or value == "":
        return
    value = str(value)
    COUNTER += 1
    CHAT_HISTORY.append(value)
    if len(CHAT_HISTORY) > 10:
        CHAT_HISTORY.pop(0)
    reversed_str = value[::-1]
    word_count = len(value.split())
    eprint(f"[dm-sdk-demo] #{COUNTER} '{value}' -> '{reversed_str}'")

    # ── Send 1: Simple text (legacy style) ──
    send_reply(msg, "text", {"content": f"**Reversed:** {reversed_str}"})

    # ── Send 2: Rich embed card ──
    send_reply(
        msg,
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
    )

    # ── Send 3: Chat-style (right side, user-like) ──
    send_reply(
        msg,
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
    )

    # ── Send 4: Stats embed every 3 messages ──
    if COUNTER % 3 == 0:
        send_reply(
            msg,
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
    eprint("[dm-sdk-demo] startup text sent")

    # Diagnostic: force-send a test message
    test_seq = msg.send("text", {"content": "🔴 TEST MESSAGE — visible?"}, from_="dm-sdk-demo")
    eprint(f"[dm-sdk-demo] test message sent, seq={test_seq}")

    # Diagnostic: show existing messages
    existing = msg.get(tag="input", widget_key=WIDGET_KEY, limit=5)
    if existing:
        eprint(f"[dm-sdk-demo] found {len(existing)} existing input(s)")
        for e in existing:
            on_new_message(msg, {
                "value": e.get("payload", {}).get("value"),
                "output_id": e.get("payload", {}).get("output_id"),
                "seq": e.get("seq"),
                "timestamp": e.get("timestamp"),
            })

    # Keep the main thread alive (dora node convention) + subscribe via WebSocket
    node = Node()
    eprint("[dm-sdk-demo] subscribing for input...")
    with msg.subscribe(tag="input", widget_key=WIDGET_KEY) as stream:
        eprint("[dm-sdk-demo] subscribed, waiting for events...")
        for event in stream:
            eprint(f"[dm-sdk-demo] GOT EVENT: {event}")
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
