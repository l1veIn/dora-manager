import json
import os
import queue
import signal
import sys
import threading
import time
from pathlib import Path

import pyarrow as pa
from dora import Node
from mss import mss
from PIL import Image

SDK_PATH = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "sdk", "python"
)
SDK_PATH = os.path.normpath(SDK_PATH)
if os.path.isdir(SDK_PATH) and SDK_PATH not in sys.path:
    sys.path.insert(0, SDK_PATH)

import dm  # noqa: E402


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
        return int(float(raw))
    except ValueError:
        return default


def env_bool(name: str, default: bool) -> bool:
    raw = env_str(name, str(default)).lower()
    return raw in {"1", "true", "yes", "on"}


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def create_node(node_id: str) -> Node:
    if env_str("DORA_NODE_CONFIG"):
        return Node()
    return Node(node_id)


def safe_stem(raw: str) -> str:
    return "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in raw).strip("_")


def capture_region(region_mode: str, monitor_index: int, x: int, y: int, width: int, height: int):
    with mss() as screen:
        if region_mode == "custom":
            region = {"left": x, "top": y, "width": width, "height": height}
        else:
            monitors = screen.monitors
            if monitor_index < 0 or monitor_index >= len(monitors):
                monitor_index = 1 if len(monitors) > 1 else 0
            region = monitors[monitor_index]
        shot = screen.grab(region)
        image = Image.frombytes("RGB", shot.size, shot.rgb)
        meta = {
            "left": region["left"],
            "top": region["top"],
            "width": region["width"],
            "height": region["height"],
            "monitor_index": monitor_index,
            "region_mode": region_mode,
        }
        return image, meta


def save_screenshot(
    output_dir: Path,
    naming: str,
    node_id: str,
    output_format: str,
    seq: int,
    region_mode: str,
    monitor_index: int,
    x: int,
    y: int,
    width: int,
    height: int,
):
    image, meta = capture_region(region_mode, monitor_index, x, y, width, height)
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    ext = "jpg" if output_format == "jpeg" else output_format
    filename = f"{safe_stem(naming.format(timestamp=timestamp, seq=f'{seq:04d}', node_id=node_id))}.{ext}"
    path = output_dir / filename
    image.save(path, format=output_format.upper())
    meta.update({
        "path": str(path),
        "content_type": "image/jpeg" if output_format == "jpeg" else "image/png",
        "captured_at": time.time(),
        "seq": seq,
    })
    return path, meta


def send_preview(msg: dm.Message, node_id: str, path: Path, run_out_dir: str, meta: dict):
    try:
        rel_path = os.path.relpath(path, run_out_dir)
        msg.send(
            "image",
            {
                "label": "Screenshot",
                "kind": "file",
                "file": rel_path,
                "embed": {
                    "author": {"name": "dm-screenshot"},
                    "title": "Screenshot captured",
                    "body": f"{meta['width']}x{meta['height']} -> {rel_path}",
                    "color": "green",
                    "status": "success",
                    "timestamp": "relative",
                },
            },
            from_=node_id,
        )
    except Exception as exc:
        print(f"[dm-screenshot] preview send failed: {exc}", file=sys.stderr, flush=True)


def register_capture_button(msg: dm.Message, widget_key: str, label: str):
    try:
        msg.widgets.register(
            key=widget_key,
            type="button",
            label=label,
            config={"value": "capture"},
        )
    except Exception as exc:
        print(
            f"[dm-screenshot] widget registration failed: {exc}",
            file=sys.stderr,
            flush=True,
        )


def subscribe_capture(msg: dm.Message, widget_key: str, events: queue.Queue):
    try:
        with msg.subscribe(tag="input", widget_key=widget_key) as stream:
            for _event in stream:
                if not RUNNING:
                    break
                events.put("widget")
    except Exception as exc:
        print(f"[dm-screenshot] widget subscribe error: {exc}", file=sys.stderr, flush=True)


def main():
    global RUNNING
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-screenshot")
    run_out_dir = env_str("DM_RUN_OUT_DIR")
    if not run_out_dir:
        raise SystemExit("DM_RUN_OUT_DIR is required")

    mode = env_str("MODE", "triggered").lower()
    relative_dir = env_str("DIR", "screenshots")
    output_dir = (Path(run_out_dir) / relative_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    output_format = env_str("OUTPUT_FORMAT", "png").lower()
    if output_format not in {"png", "jpeg"}:
        raise SystemExit("OUTPUT_FORMAT must be png or jpeg")

    region_mode = env_str("REGION_MODE", "monitor").lower()
    monitor_index = env_int("MONITOR_INDEX", 1)
    x = env_int("X", 0)
    y = env_int("Y", 0)
    width = env_int("WIDTH", 1280)
    height = env_int("HEIGHT", 720)
    naming = env_str("NAMING", "{timestamp}_{seq}")
    send_frontend_preview = env_bool("SEND_FRONTEND_PREVIEW", True)
    widget_key = env_str("WIDGET_KEY", f"{node_id}:capture")
    button_label = env_str("BUTTON_LABEL", "Capture screenshot")

    msg = dm.Message()
    node = create_node(node_id)
    events: queue.Queue = queue.Queue()
    register_capture_button(msg, widget_key, button_label)
    threading.Thread(target=subscribe_capture, args=(msg, widget_key, events), daemon=True).start()

    seq = 0

    def capture_once(source: str):
        nonlocal seq
        seq += 1
        path, meta = save_screenshot(
            output_dir,
            naming,
            node_id,
            output_format,
            seq,
            region_mode,
            monitor_index,
            x,
            y,
            width,
            height,
        )
        meta["source"] = source
        node.send_output("path", pa.array([str(path)]))
        node.send_output("meta", pa.array([json.dumps(meta)]), {"content_type": "application/json"})
        if send_frontend_preview:
            send_preview(msg, node_id, path, run_out_dir, meta)
        print(f"[dm-screenshot] captured {path}", flush=True)

    if mode == "once":
        capture_once("startup")
        return

    while RUNNING:
        try:
            source = events.get_nowait()
            capture_once(source)
        except queue.Empty:
            pass

        event = node.next(timeout=0.05)
        if event is None:
            continue
        if event["type"] == "STOP":
            break
        if event["type"] == "INPUT" and event["id"] == "trigger":
            capture_once("input")


if __name__ == "__main__":
    main()
