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


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_float(name: str, default: float) -> float:
    try:
        return float(env_str(name, str(default)))
    except ValueError:
        return default


def env_int(name: str, default: int) -> int:
    try:
        return int(float(env_str(name, str(default))))
    except ValueError:
        return default


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def normalize_output(value):
    if isinstance(value, (int, float)):
        return pa.array([float(value)], type=pa.float64())
    if isinstance(value, str):
        try:
            return pa.array([float(value)], type=pa.float64())
        except ValueError:
            return pa.array([0.0], type=pa.float64())
    if value is None:
        return pa.array([0.0], type=pa.float64())
    return pa.array([float(value)], type=pa.float64())


def poll_inputs(msg: dm.Message, node: Node, yaml_id: str, poll_interval_s: float):
    global LAST_SEQ
    while RUNNING:
        try:
            messages = msg.get(tag="input", after_seq=LAST_SEQ, limit=50)
            for item in messages:
                seq = item.get("seq", 0)
                if seq > LAST_SEQ:
                    LAST_SEQ = seq
                payload = item.get("payload", {})
                if not isinstance(payload, dict) or payload.get("to") != yaml_id:
                    continue
                node.send_output("value", normalize_output(payload.get("value")))
        except Exception as exc:
            print(f"[dm-slider] poll error: {exc}", file=sys.stderr, flush=True)
        time.sleep(poll_interval_s)


def main():
    global RUNNING
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-slider")
    label = env_str("LABEL", "Value")
    min_val = env_float("MIN_VAL", 0.0)
    max_val = env_float("MAX_VAL", 100.0)
    step = env_float("STEP", 1.0)
    default_value = env_float("DEFAULT_VALUE", 50.0)
    poll_interval_s = max(env_int("POLL_INTERVAL", 1000), 100) / 1000.0

    msg = dm.Message()
    msg.send(
        "widgets",
        {
            "label": label,
            "widgets": {
                "value": {
                    "type": "slider",
                    "label": label,
                    "min": min_val,
                    "max": max_val,
                    "step": step,
                    "default": default_value,
                }
            },
        },
        from_=node_id,
    )

    node = Node()
    poller = threading.Thread(
        target=poll_inputs, args=(msg, node, node_id, poll_interval_s), daemon=True
    )
    poller.start()

    for event in node:
        if not RUNNING:
            break


if __name__ == "__main__":
    main()
