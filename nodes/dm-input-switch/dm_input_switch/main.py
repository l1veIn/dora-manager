import json
import os
import signal
import sys
import time
from urllib.parse import urlencode, urlparse, urlunparse

import pyarrow as pa
import requests
import websocket
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
    return int(raw)


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def normalize_output(value):
    if isinstance(value, bool):
        return pa.array([value])
    if isinstance(value, str):
        return pa.array([value.lower() == "true"])
    if value is None:
        return pa.array([False])
    return pa.array([bool(value)])


def emit(server_url: str, run_id: str, node_id: str, tag: str, payload: dict):
    requests.post(
        f"{server_url}/api/runs/{run_id}/messages",
        json={
            "from": node_id,
            "tag": tag,
            "payload": payload,
            "timestamp": int(time.time()),
        },
        timeout=2,
    ).raise_for_status()


def messages_ws_url(server_url: str, run_id: str, node_id: str, since: int) -> str:
    parsed = urlparse(server_url)
    scheme = "wss" if parsed.scheme == "https" else "ws"
    path = f"/api/runs/{run_id}/messages/ws/{node_id}"
    query = urlencode({"since": since})
    return urlunparse((scheme, parsed.netloc, path, "", query, ""))


def on_message(node, widgets: dict, message: dict):
    if message.get("tag") != "input":
        return

    payload = message.get("payload", {})
    output_id = payload.get("output_id")
    if output_id in widgets:
        node.send_output(output_id, normalize_output(payload.get("value")))


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-input-switch")
    node = Node()
    run_id = env_str("DM_RUN_ID")
    server_url = env_str("DM_SERVER_URL", "http://127.0.0.1:3210")
    label = env_str("LABEL") or node_id
    poll_interval = env_int("POLL_INTERVAL", 1000)

    default_val = env_str("DEFAULT_VALUE", "false").lower() == "true"

    widgets = {
        "value": {
            "type": "switch",
            "label": label,
            "default": default_val,
        }
    }

    emit(
        server_url,
        run_id,
        node_id,
        "widgets",
        {
            "label": label,
            "widgets": widgets,
        },
    )

    since = 0
    while RUNNING:
        ws = None
        try:
            ws = websocket.create_connection(
                messages_ws_url(server_url, run_id, node_id, since),
                timeout=2,
            )
            ws.settimeout(1.0)
        except Exception as exc:
            if RUNNING:
                print(f"[{node_id}] WS connect failed: {exc}", file=sys.stderr, flush=True)
            time.sleep(max(0.1, poll_interval / 1000))
            continue

        try:
            while RUNNING:
                try:
                    raw = ws.recv()
                except websocket.WebSocketTimeoutException:
                    continue

                if not raw:
                    break

                message = json.loads(raw)
                on_message(node, widgets, message)
                since = max(since, int(message.get("seq", since)))
        except Exception as exc:
            if RUNNING:
                print(f"[{node_id}] WS receive failed: {exc}", file=sys.stderr, flush=True)
        finally:
            if ws is not None:
                try:
                    ws.close()
                except Exception:
                    pass

        time.sleep(max(0.1, poll_interval / 1000))


if __name__ == "__main__":
    main()
