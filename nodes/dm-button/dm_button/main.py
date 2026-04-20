import json
import os
import signal

import pyarrow as pa
from dora import Node


RUNNING = True


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


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
    if isinstance(value, str):
        return pa.array([value])
    if value is None:
        return pa.array(["clicked"])
    return pa.array([str(value)])


def decode_bridge_payload(value):
    if hasattr(value, "to_pylist"):
        pylist = value.to_pylist()
        if len(pylist) == 1:
            raw = pylist[0]
        elif pylist and isinstance(pylist[0], int):
            raw = bytes(pylist).decode("utf-8")
        else:
            raw = pylist
    else:
        raw = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(raw, bytes):
        raw = raw.decode("utf-8")
    if not isinstance(raw, str):
        return None
    try:
        payload = json.loads(raw)
        return payload if isinstance(payload, dict) else None
    except json.JSONDecodeError:
        return None


def main():
    global RUNNING
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    bridge_input_port = env_str("DM_BRIDGE_INPUT_PORT", "dm_bridge_input_internal")
    node = Node()

    for event in node:
        if not RUNNING:
            break
        if event["type"] != "INPUT" or event["id"] != bridge_input_port:
            continue
        payload = decode_bridge_payload(event["value"])
        if payload is None:
            continue
        node.send_output("click", normalize_output(payload.get("value")))


if __name__ == "__main__":
    main()
