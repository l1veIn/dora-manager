import json
import os
import signal
import sys
from pathlib import Path

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


def extract_value(value):
    if hasattr(value, "to_pylist"):
        pylist = value.to_pylist()
        if len(pylist) == 1:
            return pylist[0]
        return pylist

    raw = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(raw, bytes):
        return raw.decode("utf-8")
    return raw


def resolve_render(path: str, configured: str) -> str:
    if configured != "auto":
        return configured
    return EXT_TO_RENDER.get(Path(path).suffix.lower(), "text")


def resolve_inline_render(content, configured: str) -> str:
    if configured != "auto":
        return configured
    if isinstance(content, (dict, list)):
        return "json"
    return "text"


def normalize_relative(path: str, run_out_dir: str) -> str:
    if os.path.isabs(path):
        return os.path.relpath(path, run_out_dir)
    return path


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


def existing_message_path(raw_value, run_out_dir: str) -> str | None:
    if not isinstance(raw_value, str):
        return None

    candidate = raw_value.strip()
    if not candidate:
        return None

    if os.path.isabs(candidate) and os.path.exists(candidate):
        return candidate

    if run_out_dir:
        relative_candidate = os.path.join(run_out_dir, candidate)
        if os.path.exists(relative_candidate):
            return relative_candidate

    return None


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-message")
    run_out_dir = env_str("DM_RUN_OUT_DIR")
    label = env_str("LABEL") or node_id
    render_mode = env_str("RENDER", "auto")

    msg = dm.Message()
    node = Node()

    for event in node:
        if not RUNNING:
            break
        if event["type"] != "INPUT" or event["id"] != "message":
            continue

        content = extract_value(event["value"])
        existing_path = existing_message_path(content, run_out_dir)

        if existing_path:
            rel_path = normalize_relative(existing_path, run_out_dir)
            render = resolve_render(rel_path, render_mode)
            msg.send(
                render,
                {
                    "label": label,
                    "kind": "file",
                    "file": rel_path,
                },
                from_=node_id,
            )
            continue

        render = resolve_inline_render(content, render_mode)
        msg.send(
            render,
            {
                "label": label,
                "kind": "inline",
                "content": normalize_inline_content(content, render),
            },
            from_=node_id,
        )


if __name__ == "__main__":
    main()
