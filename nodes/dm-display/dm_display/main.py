import os
import sys
import time
import json
import signal
from pathlib import Path

import requests
from dora import Node


RUNNING = True


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


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def resolve_render(path: str, configured: str) -> str:
    if configured != "auto":
        return configured
    return EXT_TO_RENDER.get(Path(path).suffix.lower(), "text")


def normalize_relative(path: str, run_out_dir: str) -> str:
    if os.path.isabs(path):
        return os.path.relpath(path, run_out_dir)
    return path


def extract_path(value) -> str:
    if hasattr(value, "to_pylist"):
        pylist = value.to_pylist()
        if len(pylist) == 1:
            item = pylist[0]
            if isinstance(item, bytes):
                return item.decode("utf-8")
            return str(item)

    raw = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(raw, bytes):
        return raw.decode("utf-8")
    if isinstance(raw, list):
        return str(raw[0] if raw else "")
    if isinstance(raw, str):
        stripped = raw.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            try:
                decoded = json.loads(stripped)
                if isinstance(decoded, list):
                    return str(decoded[0] if decoded else "")
            except json.JSONDecodeError:
                pass
        return raw
    return str(raw)


def extract_data(value):
    if hasattr(value, "to_pylist"):
        pylist = value.to_pylist()
        if len(pylist) == 1:
            return pylist[0]
        return pylist

    raw = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(raw, bytes):
        return raw.decode("utf-8")
    return raw


def resolve_inline_render(content, configured: str) -> str:
    if configured != "auto":
        return configured
    if isinstance(content, (dict, list)):
        return "json"
    return "text"


def normalize_inline_content(content, render: str):
    if render == "json":
        if isinstance(content, str):
            try:
                return json.loads(content)
            except json.JSONDecodeError:
                return content
        return content
    if isinstance(content, (dict, list)):
        return json.dumps(content, ensure_ascii=False, indent=2)
    if content is None:
        return ""
    return str(content)


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


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-display")
    node = Node()
    run_id = env_str("DM_RUN_ID")
    run_out_dir = env_str("DM_RUN_OUT_DIR")
    server_url = env_str("DM_SERVER_URL", "http://127.0.0.1:3210")
    label = env_str("LABEL") or node_id
    render_mode = env_str("RENDER", "auto")

    for event in node:
        if not RUNNING:
            break
        if event["type"] != "INPUT":
            continue

        try:
            if event["id"] == "path":
                rel_path = normalize_relative(extract_path(event["value"]), run_out_dir)
                render = resolve_render(rel_path, render_mode)
                emit(
                    server_url,
                    run_id,
                    node_id,
                    render,
                    {
                        "label": label,
                        "kind": "file",
                        "file": rel_path,
                    },
                )
                print(f"[DM-IO] DISPLAY {render} -> {rel_path}", flush=True)
            elif event["id"] == "data":
                content = extract_data(event["value"])
                # Skip empty/null ticks (e.g. from timer nodes)
                if content is None or (isinstance(content, list) and len(content) == 0):
                    continue
                render = resolve_inline_render(content, render_mode)
                normalized = normalize_inline_content(content, render)
                emit(
                    server_url,
                    run_id,
                    node_id,
                    render,
                    {
                        "label": label,
                        "kind": "inline",
                        "content": normalized,
                    },
                )
                print(f"[DM-IO] DISPLAY {render} -> <inline>", flush=True)
        except Exception as exc:
            print(f"[dm-display] Server notify failed: {exc}", file=sys.stderr, flush=True)


if __name__ == "__main__":
    main()
