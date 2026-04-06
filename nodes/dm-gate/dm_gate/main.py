import os
import sys

import pyarrow as pa
from dora import Node


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_bool(name: str, default: bool = False) -> bool:
    raw = env_str(name)
    if not raw:
        return default
    return raw.lower() in {"1", "true", "yes", "on"}


def create_node(node_id: str) -> Node:
    if env_str("DORA_NODE_CONFIG"):
        return Node()
    return Node(node_id)


def normalize_bool(value) -> bool:
    py = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(py, list) and py:
        py = py[0]
    if isinstance(py, bool):
        return py
    if isinstance(py, (int, float)):
        return bool(py)
    if isinstance(py, str):
        return py.strip().lower() in {"1", "true", "yes", "on"}
    return bool(py)


def main():
    node_id = env_str("DM_NODE_ID", "dm-gate")
    node = create_node(node_id)
    emit_on_enable = env_bool("EMIT_ON_ENABLE", False)

    enabled = False
    last_value = None
    last_metadata = None

    for event in node:
        if event["type"] != "INPUT":
            continue
        if event["id"] == "enabled":
            enabled = normalize_bool(event["value"])
            print(f"[dm-gate] enabled -> {enabled}", flush=True)
            if enabled and emit_on_enable and last_value is not None:
                print("[dm-gate] forwarding cached value on enable", flush=True)
                node.send_output("value", last_value, last_metadata or {})
            continue
        if event["id"] == "value":
            last_value = event["value"]
            last_metadata = dict(event.get("metadata", {}))
            print(f"[dm-gate] received value event enabled={enabled}", flush=True)
            if enabled:
                print("[dm-gate] forwarding value", flush=True)
                try:
                    node.send_output("value", event["value"], last_metadata)
                except Exception as exc:
                    # Timer sources can yield scalar timestamp payloads that are not directly
                    # representable by this generic gate. Fall back to a null pulse while
                    # preserving metadata so trigger-style downstream nodes still work.
                    print(f"[dm-gate] fallback pulse due to send error: {exc}", flush=True)
                    node.send_output("value", pa.nulls(1), last_metadata)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[dm-gate] {exc}", file=sys.stderr, flush=True)
        raise
