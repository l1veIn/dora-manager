import json
import os
import sys
import time

import pyarrow as pa
from dora import Node


INPUT_IDS = ("a", "b", "c", "d")


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


def env_csv(name: str, default: str) -> list[str]:
    raw = env_str(name, default)
    return [item.strip() for item in raw.split(",") if item.strip()]


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


def emit(node: Node, state: dict[str, bool], require_all_seen: bool, expected_inputs: list[str]):
    seen = {key: state[key] for key in expected_inputs if key in state}
    considered = [state[key] for key in expected_inputs if key in state]
    ready = bool(considered) and all(considered)
    if require_all_seen and any(key not in state for key in expected_inputs):
        ready = False
    details = {
        "kind": "and",
        "ready": ready,
        "checked_at": time.time(),
        "inputs": seen,
        "require_all_seen": require_all_seen,
        "expected_inputs": expected_inputs,
    }
    node.send_output("ok", pa.array([ready]))
    node.send_output("details", pa.array([json.dumps(details)]), {"content_type": "application/json"})


def main():
    node_id = env_str("DM_NODE_ID", "dm-and")
    node = create_node(node_id)
    require_all_seen = env_bool("REQUIRE_ALL_SEEN", True)
    expected_inputs = [item for item in env_csv("EXPECTED_INPUTS", "a,b") if item in INPUT_IDS]
    if not expected_inputs:
        expected_inputs = ["a", "b"]
    state: dict[str, bool] = {}

    for event in node:
        if event["type"] != "INPUT":
            continue
        event_id = event["id"]
        if event_id not in INPUT_IDS:
            continue
        state[event_id] = normalize_bool(event["value"])
        emit(node, state, require_all_seen, expected_inputs)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[dm-and] {exc}", file=sys.stderr, flush=True)
        raise
