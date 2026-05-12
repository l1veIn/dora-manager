import json
import os
import queue
import signal
import sys
import threading
import time

import pyarrow as pa
from dora import Node

# Add SDK to path for development from a source checkout.
SDK_PATH = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "sdk", "python"
)
SDK_PATH = os.path.normpath(SDK_PATH)
if os.path.isdir(SDK_PATH) and SDK_PATH not in sys.path:
    sys.path.insert(0, SDK_PATH)

import dm  # noqa: E402


RUNNING = True
ENABLED = True
LAST_SEQ = 0

MODIFIERS = {
    "cmd": "<cmd>",
    "command": "<cmd>",
    "super": "<cmd>",
    "meta": "<cmd>",
    "ctrl": "<ctrl>",
    "control": "<ctrl>",
    "alt": "<alt>",
    "option": "<alt>",
    "shift": "<shift>",
}

SPECIAL_KEYS = {
    "enter": "<enter>",
    "return": "<enter>",
    "esc": "<esc>",
    "escape": "<esc>",
    "space": "<space>",
    "tab": "<tab>",
    "backspace": "<backspace>",
    "delete": "<delete>",
    "up": "<up>",
    "down": "<down>",
    "left": "<left>",
    "right": "<right>",
}


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_bool(name: str, default: bool) -> bool:
    raw = env_str(name, str(default)).lower()
    return raw in {"1", "true", "yes", "on", "enabled"}


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def normalize_combo(combo: str) -> str:
    parts = [part.strip().lower() for part in combo.replace("-", "+").split("+")]
    normalized: list[str] = []
    for part in parts:
        if not part:
            continue
        if part in MODIFIERS:
            normalized.append(MODIFIERS[part])
        elif part in SPECIAL_KEYS:
            normalized.append(SPECIAL_KEYS[part])
        elif part.startswith("f") and part[1:].isdigit():
            normalized.append(f"<{part}>")
        else:
            normalized.append(part)
    if not normalized:
        raise ValueError("hotkey combo is empty")
    return "+".join(normalized)


def create_node(node_id: str) -> Node:
    if env_str("DORA_NODE_CONFIG"):
        return Node()
    return Node(node_id)


def send_status(msg: dm.Message, node_id: str, status: str, title: str, body: str, color: str):
    try:
        msg.send(
            "text",
            {
                "content": body,
                "embed": {
                    "author": {"name": "dm-hotkey"},
                    "title": title,
                    "body": body,
                    "color": color,
                    "status": status,
                    "side": "left",
                    "width": "compact",
                    "timestamp": "relative",
                },
            },
            from_=node_id,
        )
    except Exception as exc:
        print(f"[dm-hotkey] failed to send status: {exc}", file=sys.stderr, flush=True)


def register_widgets(msg: dm.Message, widget_key: str, label: str, enabled: bool):
    msg.widgets.register(
        key=widget_key,
        type="switch",
        label=label,
        config={
            "default": enabled,
            "description": "Enable or disable this global hotkey source.",
        },
    )


def input_value(event: dict):
    payload = event.get("payload")
    if isinstance(payload, dict):
        return payload.get("value")
    return event.get("value")


def subscribe_enabled(msg: dm.Message, widget_key: str):
    global ENABLED
    try:
        with msg.subscribe(tag="input", widget_key=widget_key) as stream:
            for event in stream:
                if not RUNNING:
                    break
                value = input_value(event)
                if value is None:
                    continue
                ENABLED = str(value).lower() in {"1", "true", "yes", "on"}
                print(f"[dm-hotkey] enabled={ENABLED}", flush=True)
    except Exception as exc:
        print(f"[dm-hotkey] widget subscribe error: {exc}", file=sys.stderr, flush=True)


def build_listener(normalized_combo: str, events: queue.Queue):
    from pynput import keyboard

    hotkey = keyboard.HotKey(
        keyboard.HotKey.parse(normalized_combo),
        lambda: events.put({"type": "hotkey", "ts": time.time()}),
    )

    def for_canonical(callback):
        return lambda key: callback(listener.canonical(key))

    listener = keyboard.Listener(
        on_press=for_canonical(hotkey.press),
        on_release=for_canonical(hotkey.release),
    )
    return listener


def emit_hotkey(node: Node, combo: str, normalized_combo: str, label: str):
    payload = {
        "combo": combo,
        "normalized_combo": normalized_combo,
        "label": label,
        "triggered_at": time.time(),
    }
    node.send_output("trigger", pa.array([combo]))
    node.send_output(
        "event",
        pa.array([json.dumps(payload)]),
        {"content_type": "application/json"},
    )


def main():
    global RUNNING, ENABLED
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-hotkey")
    combo = env_str("COMBO", "cmd+shift+2")
    label = env_str("LABEL", f"Hotkey {combo}")
    widget_key = env_str("WIDGET_KEY", f"{node_id}:enabled")
    send_frontend_status = env_bool("SEND_FRONTEND_STATUS", True)
    ENABLED = env_bool("ENABLED", True)

    msg = dm.Message()
    node = create_node(node_id)
    normalized_combo = normalize_combo(combo)
    register_widgets(msg, widget_key, f"{label} enabled", ENABLED)

    if send_frontend_status:
        send_status(
            msg,
            node_id,
            "info",
            "Hotkey armed",
            f"{label} is listening for `{combo}`.",
            "blue",
        )

    threading.Thread(target=subscribe_enabled, args=(msg, widget_key), daemon=True).start()

    events: queue.Queue = queue.Queue()
    try:
        listener = build_listener(normalized_combo, events)
        listener.start()
    except Exception as exc:
        detail = str(exc) or exc.__class__.__name__
        print(f"[dm-hotkey] listener failed: {detail}", file=sys.stderr, flush=True)
        if send_frontend_status:
            send_status(
                msg,
                node_id,
                "error",
                "Hotkey unavailable",
                f"Could not listen for `{combo}`: {detail}",
                "red",
            )
        raise

    print(f"[dm-hotkey] listening combo={combo} normalized={normalized_combo}", flush=True)

    try:
        while RUNNING:
            try:
                event = events.get(timeout=0.1)
            except queue.Empty:
                continue
            if event.get("type") != "hotkey" or not ENABLED:
                continue
            emit_hotkey(node, combo, normalized_combo, label)
            if send_frontend_status:
                send_status(
                    msg,
                    node_id,
                    "success",
                    "Hotkey triggered",
                    f"{label} fired `{combo}`.",
                    "green",
                )
    finally:
        RUNNING = False
        listener.stop()
        print("[dm-hotkey] stopped", flush=True)


if __name__ == "__main__":
    main()
