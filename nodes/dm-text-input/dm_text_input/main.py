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
    if isinstance(value, str):
        return pa.array([value])
    if value is None:
        return pa.array([""])
    return pa.array([str(value)])


def diag_value(v):
    """Return a short diagnostic string for an arrow value."""
    vtype = type(v).__name__
    methods = []
    for m in ("to_pylist", "as_py", "as_buffer"):
        if hasattr(v, m):
            methods.append(m)
    # try to extract a preview
    preview = ""
    try:
        if hasattr(v, "to_pylist"):
            pl = v.to_pylist()
            preview = repr(pl[:3]) if isinstance(pl, list) else repr(pl)
        elif hasattr(v, "as_py"):
            preview = repr(v.as_py())
    except Exception as e:
        preview = f"<err: {e}>"
    return f"type={vtype} methods={methods} preview={preview}"


def decode_bridge_payload(value):
    if hasattr(value, "to_pylist"):
        pylist = value.to_pylist()
        if len(pylist) == 1:
            raw = pylist[0]
        elif pylist and isinstance(pylist[0], int):
            # UInt8Array → list of byte values → reconstruct string
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
    print(f"[dm-text-input] starting, bridge_input_port={bridge_input_port!r}", flush=True)
    node = Node()

    for event in node:
        if not RUNNING:
            break
        if event["type"] != "INPUT":
            continue
        eid = event["id"]
        print(f"[dm-text-input] event id={eid!r} match={eid == bridge_input_port} value_diag={diag_value(event['value'])}", flush=True)
        if eid != bridge_input_port:
            continue
        payload = decode_bridge_payload(event["value"])
        print(f"[dm-text-input] decoded payload={payload!r}", flush=True)
        if payload is None:
            continue
        out_val = payload.get("value")
        node.send_output("value", normalize_output(out_val))
        print(f"[dm-text-input] forwarded value={out_val!r}", flush=True)


if __name__ == "__main__":
    main()
