import os
import sys
import time
from pathlib import Path

import requests
from dora import Node


EXT_TO_RENDER = {
    ".log": "text",
    ".txt": "text",
    ".json": "json",
    ".md": "markdown",
    ".png": "image",
    ".jpg": "image",
    ".jpeg": "image",
    ".wav": "audio",
    ".mp3": "audio",
    ".mp4": "video",
}


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


def env_int(name: str, default: int) -> int:
    raw = env_str(name)
    if not raw:
        return default
    return int(raw)


def resolve_render(path: str, configured: str) -> str:
    if configured != "auto":
        return configured
    return EXT_TO_RENDER.get(Path(path).suffix.lower(), "text")


def normalize_relative(path: str, run_out_dir: str) -> str:
    if os.path.isabs(path):
        return os.path.relpath(path, run_out_dir)
    return path


def notify(server_url: str, run_id: str, node_id: str, label: str, rel_path: str, render: str, tail: bool, max_lines: int):
    requests.post(
        f"{server_url}/api/runs/{run_id}/interaction/display",
        json={
            "node_id": node_id,
            "label": label,
            "file": rel_path,
            "render": render,
            "tail": tail,
            "max_lines": max_lines,
            "timestamp": int(time.time()),
        },
        timeout=2,
    ).raise_for_status()


def main():
    node = Node()
    run_id = env_str("DM_RUN_ID")
    run_out_dir = env_str("DM_RUN_OUT_DIR")
    server_url = env_str("DM_SERVER_URL", "http://127.0.0.1:3210")
    label = env_str("LABEL") or node.node_id()
    render_mode = env_str("RENDER", "auto")
    tail = env_bool("TAIL", True)
    max_lines = env_int("MAX_LINES", 500)
    source = env_str("SOURCE")
    poll_interval = env_int("POLL_INTERVAL", 2000)

    if source:
        full_path = Path(run_out_dir) / source
        last_mtime = -1.0
        while True:
            if full_path.exists():
                mtime = full_path.stat().st_mtime
                if mtime > last_mtime:
                    last_mtime = mtime
                    render = resolve_render(source, render_mode)
                    notify(server_url, run_id, node.node_id(), label, source, render, tail, max_lines)
                    print(f"[DM-IO] DISPLAY {render} -> {source}", flush=True)
            time.sleep(max(0.1, poll_interval / 1000))

    for event in node:
        if event["type"] != "INPUT" or event["id"] != "path":
            continue

        raw = event["value"].as_py() if hasattr(event["value"], "as_py") else event["value"]
        if isinstance(raw, bytes):
            raw = raw.decode("utf-8")
        if isinstance(raw, list):
            raw = raw[0] if raw else ""
        rel_path = normalize_relative(str(raw), run_out_dir)
        render = resolve_render(rel_path, render_mode)

        try:
            notify(server_url, run_id, node.node_id(), label, rel_path, render, tail, max_lines)
            print(f"[DM-IO] DISPLAY {render} -> {rel_path}", flush=True)
        except Exception as exc:
            print(f"[dm-display] Server notify failed: {exc}", file=sys.stderr, flush=True)


if __name__ == "__main__":
    main()
