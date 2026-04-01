import base64
import json
import os
import sys
import time

import pyarrow as pa
import requests
from dora import Node


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


def main():
    node = Node()
    run_id = env_str("DM_RUN_ID")
    server_url = env_str("DM_SERVER_URL", "http://127.0.0.1:3210")
    label = env_str("LABEL") or node.node_id()
    poll_interval = env_int("POLL_INTERVAL", 1000)

    widgets_raw = env_str("WIDGETS", "{}")
    try:
        widgets = json.loads(widgets_raw) if widgets_raw else {}
    except json.JSONDecodeError as exc:
        raise SystemExit(f"Invalid WIDGETS config: {exc}")

    requests.post(
        f"{server_url}/api/runs/{run_id}/interaction/input/register",
        json={
            "node_id": node.node_id(),
            "label": label,
            "widgets": widgets,
        },
        timeout=2,
    ).raise_for_status()

    since = 0
    while True:
        try:
            response = requests.get(
                f"{server_url}/api/runs/{run_id}/interaction/input/claim/{node.node_id()}",
                params={"since": since},
                timeout=5,
            )
            response.raise_for_status()
            payload = response.json()
        except Exception as exc:
            print(f"[dm-input] Claim failed: {exc}", file=sys.stderr, flush=True)
            time.sleep(max(0.1, poll_interval / 1000))
            continue

        for event in payload.get("events", []):
            output_id = event["output_id"]
            widget_def = widgets.get(output_id, {})
            value = decode_event_value(widget_def, event["value"])
            node.send_output(output_id, normalize_output(value))
        since = payload.get("next_seq", since)
        time.sleep(max(0.1, poll_interval / 1000))


if __name__ == "__main__":
    main()
