import json
import os
import platform
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
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    try:
        return int(raw)
    except ValueError:
        return default


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def create_node(node_id: str) -> Node:
    if env_str("DORA_NODE_CONFIG"):
        return Node()
    return Node(node_id)


def capture_args(system_name: str, width: int, height: int, target: str) -> list[str]:
    size = f"{width}x{height}"
    if system_name == "Darwin":
        # avfoundation screen capture can stall when forcing -video_size.
        source = target if target != "desktop" else "0:none"
        return ["-f", "avfoundation", "-framerate", "1", "-i", source]
    if system_name == "Linux":
        source = target if target != "desktop" else env_str("DISPLAY", ":0.0")
        return ["-f", "x11grab", "-framerate", "1", "-video_size", size, "-i", source]
    if system_name == "Windows":
        source = target if target != "desktop" else "desktop"
        return ["-f", "gdigrab", "-framerate", "1", "-video_size", size, "-i", source]
    raise RuntimeError(f"Unsupported platform for screen capture: {system_name}")


def build_ffmpeg_command(
    ffmpeg_path: str,
    output_format: str,
    width: int,
    height: int,
    target: str,
) -> list[str]:
    codec = "png" if output_format == "png" else "mjpeg"
    command = [
        ffmpeg_path,
        "-hide_banner",
        "-loglevel",
        "warning",
        *capture_args(platform.system(), width, height, target),
    ]
    if platform.system() == "Darwin":
        command.extend(["-vf", f"scale={width}:{height}"])
    command.extend([
        "-frames:v",
        "1",
        "-f",
        "image2pipe",
        "-vcodec",
        codec,
        "-",
    ])
    return command


def capture_frame(ffmpeg_path: str, output_format: str, width: int, height: int, target: str) -> bytes:
    command = build_ffmpeg_command(ffmpeg_path, output_format, width, height, target)
    result = subprocess.run(command, capture_output=True)
    if result.returncode != 0:
        stderr = result.stderr.decode(errors="ignore").strip()
        raise RuntimeError(stderr or "ffmpeg screen capture failed")
    return result.stdout


def emit_frame(
    node: Node,
    ffmpeg_path: str,
    output_format: str,
    width: int,
    height: int,
    target: str,
    mode: str,
):
    data = capture_frame(ffmpeg_path, output_format, width, height, target)
    content_type = "image/png" if output_format == "png" else "image/jpeg"
    filename = "frame.png" if output_format == "png" else "frame.jpg"
    metadata = {
        "content_type": content_type,
        "filename": filename,
        "width": width,
        "height": height,
        "capture_target": target,
        "mode": mode,
    }
    print(
        f"[dm-screen-capture] emitted frame {len(data)} bytes {width}x{height} {content_type}",
        flush=True,
    )
    node.send_output("frame", pa.array(data, type=pa.uint8()), metadata)
    node.send_output(
        "meta",
        pa.array(
            [
                json.dumps(
                    {
                        "content_type": content_type,
                        "filename": filename,
                        "width": width,
                        "height": height,
                        "capture_target": target,
                        "mode": mode,
                        "byte_size": len(data),
                        "captured_at": time.time(),
                    }
                )
            ]
        ),
        {"content_type": "application/json"},
    )


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-screen-capture")
    ffmpeg_path = env_str("FFMPEG_PATH", "ffmpeg")
    output_format = env_str("OUTPUT_FORMAT", "png").lower()
    width = env_int("WIDTH", 1280)
    height = env_int("HEIGHT", 720)
    interval_sec = env_int("INTERVAL_SEC", 1)
    mode = env_str("MODE", "repeat").lower()
    capture_target = env_str("CAPTURE_TARGET", "desktop")
    print(
        f"[dm-screen-capture] init node_id={node_id} has_dora_config={bool(env_str('DORA_NODE_CONFIG'))}",
        flush=True,
    )
    node = create_node(node_id)
    print(
        f"[dm-screen-capture] start node_id={node_id} mode={mode} format={output_format} target={capture_target} size={width}x{height} ffmpeg={ffmpeg_path}",
        flush=True,
    )

    if output_format not in {"png", "jpeg"}:
        raise SystemExit("Unsupported OUTPUT_FORMAT. Supported: png, jpeg.")
    if mode not in {"once", "repeat", "triggered"}:
        raise SystemExit("Unsupported MODE. Supported: once, repeat, triggered.")

    if mode == "once":
        emit_frame(node, ffmpeg_path, output_format, width, height, capture_target, mode)
        return

    if mode == "triggered":
        while RUNNING:
            event = node.next()
            if event is None or event["type"] == "STOP":
                return
            if event["type"] == "INPUT" and event["id"] == "trigger":
                print("[dm-screen-capture] received trigger", flush=True)
                emit_frame(node, ffmpeg_path, output_format, width, height, capture_target, mode)
        return

    while RUNNING:
        emit_frame(node, ffmpeg_path, output_format, width, height, capture_target, mode)
        for _ in range(max(1, interval_sec * 10)):
            if not RUNNING:
                break
            time.sleep(0.1)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[dm-screen-capture] {exc}", file=sys.stderr, flush=True)
        raise
