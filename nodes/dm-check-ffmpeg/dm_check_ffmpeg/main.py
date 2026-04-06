import json
import os
import shutil
import signal
import subprocess
import sys
import time

import pyarrow as pa
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


def check_ffmpeg(ffmpeg_path: str) -> dict:
    resolved = shutil.which(ffmpeg_path) if os.path.basename(ffmpeg_path) == ffmpeg_path else ffmpeg_path
    details = {
        "kind": "ffmpeg",
        "ready": False,
        "checked_at": time.time(),
        "path": resolved,
        "requested_path": ffmpeg_path,
    }
    if not resolved or not os.path.exists(resolved):
        details["error"] = "ffmpeg not found"
        return details

    try:
        result = subprocess.run(
            [resolved, "-version"],
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )
    except Exception as exc:
        details["error"] = str(exc)
        return details

    details["ready"] = result.returncode == 0
    first_line = (result.stdout or result.stderr or "").splitlines()
    details["version"] = first_line[0] if first_line else ""
    if result.returncode != 0:
        details["error"] = details["version"] or "ffmpeg command failed"
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

    node_id = env_str("DM_NODE_ID", "dm-check-ffmpeg")
    node = create_node(node_id)
    ffmpeg_path = env_str("FFMPEG_PATH", "ffmpeg")
    interval_sec = env_int("INTERVAL_SEC", 5)
    mode = env_str("MODE", "once").lower()

    if mode not in {"once", "repeat", "triggered"}:
        raise SystemExit("Unsupported MODE. Supported: once, repeat, triggered.")

    if mode == "once":
        emit(node, check_ffmpeg(ffmpeg_path))
        return

    if mode == "triggered":
        while RUNNING:
            event = node.next()
            if event is None or event["type"] == "STOP":
                return
            if event["type"] == "INPUT" and event["id"] == "trigger":
                emit(node, check_ffmpeg(ffmpeg_path))
        return

    while RUNNING:
        emit(node, check_ffmpeg(ffmpeg_path))
        for _ in range(max(1, interval_sec * 10)):
            if not RUNNING:
                break
            time.sleep(0.1)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[dm-check-ffmpeg] {exc}", file=sys.stderr, flush=True)
        raise
