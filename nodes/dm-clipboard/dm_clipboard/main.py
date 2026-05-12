import os
import signal
import sys
import time

import pyarrow as pa
import pyperclip
from dora import Node

SDK_PATH = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "sdk", "python"
)
SDK_PATH = os.path.normpath(SDK_PATH)
if os.path.isdir(SDK_PATH) and SDK_PATH not in sys.path:
    sys.path.insert(0, SDK_PATH)

import dm  # noqa: E402


RUNNING = True


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_bool(name: str, default: bool) -> bool:
    raw = env_str(name, str(default)).lower()
    return raw in {"1", "true", "yes", "on"}


def env_float(name: str, default: float) -> float:
    raw = env_str(name, str(default))
    try:
        return float(raw)
    except ValueError:
        return default


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def extract_text(value) -> str:
    if hasattr(value, "to_pylist"):
        values = value.to_pylist()
        if len(values) == 1:
            return extract_text(values[0])
        return "\n".join(extract_text(item) for item in values)
    raw = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(raw, bytes):
        return raw.decode("utf-8", errors="replace")
    if raw is None:
        return ""
    return str(raw)


def send_status(msg: dm.Message, node_id: str, title: str, body: str, color: str):
    try:
        msg.send(
            "text",
            {
                "content": body,
                "embed": {
                    "author": {"name": "dm-clipboard"},
                    "title": title,
                    "body": body,
                    "color": color,
                    "timestamp": "relative",
                    "width": "compact",
                },
            },
            from_=node_id,
        )
    except Exception as exc:
        print(f"[dm-clipboard] status send failed: {exc}", file=sys.stderr, flush=True)


def main():
    global RUNNING
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-clipboard")
    mode = env_str("MODE", "write").lower()
    emit_on_start = env_bool("EMIT_ON_START", False)
    monitor_interval = max(env_float("MONITOR_INTERVAL", 0.5), 0.1)
    send_frontend_status = env_bool("SEND_FRONTEND_STATUS", True)

    msg = dm.Message()
    node = Node()

    def emit_current(source: str):
        value = pyperclip.paste()
        node.send_output("value", pa.array([value]))
        if send_frontend_status:
            send_status(msg, node_id, "Clipboard read", f"Clipboard read via {source}.", "blue")
        return value

    if emit_on_start or mode in {"read", "monitor"}:
        last_value = emit_current("startup")
    else:
        last_value = ""

    if mode == "read":
        return

    while RUNNING:
        event = node.next(timeout=monitor_interval if mode == "monitor" else 0.05)
        if event is not None:
            if event["type"] == "STOP":
                break
            if event["type"] == "INPUT" and event["id"] == "text":
                text = extract_text(event["value"])
                pyperclip.copy(text)
                node.send_output("value", pa.array([text]))
                last_value = text
                if send_frontend_status:
                    send_status(msg, node_id, "Copied to clipboard", text[:240], "green")

        if mode == "monitor":
            try:
                current = pyperclip.paste()
            except Exception as exc:
                print(f"[dm-clipboard] paste failed: {exc}", file=sys.stderr, flush=True)
                time.sleep(monitor_interval)
                continue
            if current != last_value:
                last_value = current
                node.send_output("value", pa.array([current]))
                if send_frontend_status:
                    send_status(msg, node_id, "Clipboard changed", current[:240], "blue")


if __name__ == "__main__":
    main()
