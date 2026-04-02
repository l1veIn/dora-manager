import base64
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
    if isinstance(value, int):
        return pa.array([value], type=pa.int64())
    if isinstance(value, float):
        return pa.array([value], type=pa.float64())
    if isinstance(value, list):
        return pa.array([value])
    if isinstance(value, dict):
        return pa.array([json.dumps(value, ensure_ascii=False)])
    if isinstance(value, str):
        return pa.array([value])
    if value is None:
        return pa.array([None], type=pa.null())
    return pa.array([str(value)])


def decode_event_value(widget_def, value):
    widget_type = (widget_def or {}).get("type", "")
    if widget_type == "file" and isinstance(value, str):
        return base64.b64decode(value)
    return value


def server_ws_url(server_url: str, run_id: str, node_id: str, since: int) -> str:
    parsed = urlparse(server_url)
    scheme = "wss" if parsed.scheme == "https" else "ws"
    path = f"/api/runs/{run_id}/interaction/input/ws/{node_id}"
    query = urlencode({"since": since})
    return urlunparse((scheme, parsed.netloc, path, "", query, ""))


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-input")
    node = Node()
    run_id = env_str("DM_RUN_ID")
    server_url = env_str("DM_SERVER_URL", "http://127.0.0.1:3210")
    label = env_str("LABEL") or node_id
    poll_interval = env_int("POLL_INTERVAL", 1000)

    widgets_raw = env_str("WIDGETS", "{}")
    try:
        widgets = json.loads(widgets_raw) if widgets_raw else {}
    except json.JSONDecodeError as exc:
        raise SystemExit(f"Invalid WIDGETS config: {exc}")

    requests.post(
        f"{server_url}/api/runs/{run_id}/interaction/input/register",
        json={
            "node_id": node_id,
            "label": label,
            "widgets": widgets,
        },
        timeout=2,
    ).raise_for_status()

    since = 0
    while RUNNING:
        ws = None
        try:
            ws = websocket.create_connection(
                server_ws_url(server_url, run_id, node_id, since),
                timeout=2,
            )
            ws.settimeout(1.0)
        except Exception as exc:
            if RUNNING:
                print(f"[dm-input] WS connect failed: {exc}", file=sys.stderr, flush=True)
            time.sleep(max(0.1, poll_interval / 1000))
            continue

        try:
            while RUNNING:
                try:
                    raw = ws.recv()
                except websocket.WebSocketTimeoutException:
                    continue

                if not raw:
                    raise RuntimeError("input event websocket closed")

                payload = json.loads(raw)
                if payload.get("type") != "input.event":
                    continue

                event = payload.get("event", {})
                output_id = event["output_id"]
                widget_def = widgets.get(output_id, {})
                value = decode_event_value(widget_def, event["value"])
                node.send_output(output_id, normalize_output(value))
                since = max(since, int(event.get("seq", since)))
        except Exception as exc:
            if RUNNING:
                print(f"[dm-input] WS receive failed: {exc}", file=sys.stderr, flush=True)
        finally:
            if ws is not None:
                try:
                    ws.close()
                except Exception:
                    pass

        time.sleep(max(0.1, poll_interval / 1000))


if __name__ == "__main__":
    main()
