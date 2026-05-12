import json
import os
import platform
import signal
import subprocess
import sys
import webbrowser

import pyarrow as pa
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


def open_target(target: str, reveal: bool):
    if target.startswith(("http://", "https://")):
        if not webbrowser.open(target):
            raise RuntimeError("webbrowser.open returned false")
        return
    system = platform.system()
    if system == "Darwin":
        command = ["open"]
        if reveal:
            command.append("-R")
        command.append(target)
        subprocess.run(command, check=True)
        return
    if system == "Linux":
        subprocess.run(["xdg-open", target], check=True)
        return
    if system == "Windows":
        os.startfile(target)  # type: ignore[attr-defined]
        return
    raise RuntimeError(f"unsupported open platform: {system}")


def send_frontend(msg: dm.Message, node_id: str, title: str, body: str, ok: bool):
    try:
        msg.send(
            "text",
            {
                "content": body,
                "embed": {
                    "author": {"name": "dm-open"},
                    "title": title,
                    "body": body,
                    "color": "green" if ok else "red",
                    "status": "success" if ok else "error",
                    "timestamp": "relative",
                    "width": "compact",
                },
            },
            from_=node_id,
        )
    except Exception as exc:
        print(f"[dm-open] frontend send failed: {exc}", file=sys.stderr, flush=True)


def main():
    global RUNNING
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-open")
    default_target = env_str("TARGET")
    reveal = env_bool("REVEAL", False)
    send_frontend_status = env_bool("SEND_FRONTEND_STATUS", True)

    msg = dm.Message()
    node = Node()

    def handle_target(target: str):
        ok = True
        error = ""
        try:
            open_target(target, reveal)
        except Exception as exc:
            ok = False
            error = str(exc)
            print(f"[dm-open] open failed: {error}", file=sys.stderr, flush=True)
        payload = {
            "ok": ok,
            "target": target,
            "error": error,
        }
        node.send_output("status", pa.array([json.dumps(payload)]), {"content_type": "application/json"})
        if send_frontend_status:
            send_frontend(msg, node_id, "Opened" if ok else "Open failed", target if ok else error, ok)

    if default_target:
        handle_target(default_target)

    for event in node:
        if not RUNNING:
            break
        if event["type"] != "INPUT" or event["id"] != "target":
            continue
        target = extract_text(event["value"]).strip()
        if target:
            handle_target(target)


if __name__ == "__main__":
    main()
