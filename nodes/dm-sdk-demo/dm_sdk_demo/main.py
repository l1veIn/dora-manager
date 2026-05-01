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
import threading
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
LAST_SEQ = 0
MSG_LOCK = threading.Lock()


def handle_signal(signum, frame):
    global RUNNING
    RUNNING = False


signal.signal(signal.SIGINT, handle_signal)
signal.signal(signal.SIGTERM, handle_signal)


def on_new_message(msg: dm.Message, payload: dict):
    """Called when a new input message arrives."""
    value = payload.get("value", "")
    if not value:
        return
    reversed_str = value[::-1]
    eprint(f"[dm-sdk-demo] reversed '{value}' -> '{reversed_str}'")
    msg.send("text", {"content": f"**Reversed:** {reversed_str}"}, from_="dm-sdk-demo")


def subscriber_loop(msg: dm.Message):
    """Background thread: listen for input messages via SDK pull loop."""
    global LAST_SEQ, RUNNING
    while RUNNING:
        try:
            with MSG_LOCK:
                new_msgs = msg.get(tag="input", after_seq=LAST_SEQ, limit=20)
                for m in new_msgs:
                    seq = m.get("seq", 0)
                    if seq > LAST_SEQ:
                        LAST_SEQ = seq
                    payload = m.get("payload", {})
                    target = payload.get("to", "")
                    if target != "input-text":
                        continue
                    on_new_message(msg, payload)
        except Exception as e:
            eprint(f"[dm-sdk-demo] poll error: {e}")
        time.sleep(0.5)


def main():
    global RUNNING

    # Initialize SDK
    msg = dm.Message()
    eprint(f"[dm-sdk-demo] starting, run_id={msg.run_id}")

    # Register widgets in the Web UI
    # This replicates what bridge.rs does on init via Unix socket
    msg.send(
        "widgets",
        {
            "label": "SDK Demo",
            "widgets": {
                "value": {
                    "type": "input",
                    "label": "Text to reverse",
                    "default": "",
                    "placeholder": "Type something and press Enter...",
                }
            },
        },
        from_="dm-sdk-demo",
    )

    # Register widget metadata for the UI
    msg.send(
        "widget-register",
        {
            "node_id": "input-text",
            "type": "text-input",
            "label": "Text to reverse",
            "placeholder": "Type something and press Enter...",
        },
        from_="dm-sdk-demo",
    )

    # Send a startup message
    msg.send("text", {"content": "✅ SDK Demo node started — type something to reverse"}, from_="dm-sdk-demo")

    # Start background poller for input messages
    poller = threading.Thread(target=subscriber_loop, args=(msg,), daemon=True)
    poller.start()

    # Keep the main thread alive (dora node convention)
    node = Node()
    for event in node:
        if not RUNNING:
            break
        # We could process dora events here, but this demo is SDK-only
        time.sleep(0.1)

    RUNNING = False
    eprint("[dm-sdk-demo] shutting down")
    msg.send("text", {"content": "⏹️ SDK Demo stopped"}, from_="dm-sdk-demo")


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


if __name__ == "__main__":
    main()
