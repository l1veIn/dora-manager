import json
import os
import signal
import sys
import time

import pyarrow as pa
import requests
from dora import Node


RUNNING = True


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_int(name: str, default: int) -> int:
    raw = env_str(name)
    if not raw:
        return default
    try:
        return int(raw)
    except ValueError:
        return default


def create_node(node_id: str) -> Node:
    if env_str("DORA_NODE_CONFIG"):
        return Node()
    return Node(node_id)


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def check_media(server_url: str) -> dict:
    details = {
        "kind": "media_backend",
        "ready": False,
        "checked_at": time.time(),
        "server_url": server_url,
    }
    try:
        response = requests.get(f"{server_url}/api/media/status", timeout=5)
        response.raise_for_status()
        payload = response.json()
        details.update(payload)
        details["ready"] = payload.get("status") == "ready"
        return details
    except Exception as exc:
        details["error"] = str(exc)
        return details


def emit(node: Node, details: dict):
    node.send_output("ok", pa.array([bool(details.get("ready", False))]))
    node.send_output(
        "details",
        pa.array([json.dumps(details)]),
        {"content_type": "application/json"},
    )


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-check-media-backend")
    node = create_node(node_id)
    server_url = env_str("DM_SERVER_URL", "http://127.0.0.1:3210")
    interval_sec = env_int("INTERVAL_SEC", 5)
    mode = env_str("MODE", "once").lower()

    if mode not in {"once", "repeat", "triggered"}:
        raise SystemExit("Unsupported MODE. Supported: once, repeat, triggered.")

    if mode == "once":
        emit(node, check_media(server_url))
        return

    if mode == "triggered":
        while RUNNING:
            event = node.next()
            if event is None or event["type"] == "STOP":
                return
            if event["type"] == "INPUT" and event["id"] == "trigger":
                emit(node, check_media(server_url))
        return

    while RUNNING:
        emit(node, check_media(server_url))
        for _ in range(max(1, interval_sec * 10)):
            if not RUNNING:
                break
            time.sleep(0.1)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[dm-check-media-backend] {exc}", file=sys.stderr, flush=True)
        raise
