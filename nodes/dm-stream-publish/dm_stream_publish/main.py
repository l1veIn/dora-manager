import json
import os
import signal
import subprocess
import sys
import time
from typing import Optional
from urllib.parse import quote
import re

import pyarrow as pa
import requests
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


def get_media_status(server_url: str) -> dict:
    response = requests.get(f"{server_url}/api/media/status", timeout=5)
    response.raise_for_status()
    return response.json()


def wait_for_mediamtx_path(media: dict, path: str, timeout_sec: float = 8.0) -> bool:
    api_port = media.get("api_port")
    host = media.get("host")
    if not api_port or not host:
        return False

    deadline = time.time() + timeout_sec
    encoded_path = quote(path, safe="")
    while time.time() < deadline:
        try:
            response = requests.get(
                f"http://{host}:{api_port}/v3/paths/get/{encoded_path}",
                timeout=1.0,
            )
            if response.ok:
                payload = response.json()
                if payload.get("ready") or payload.get("available") or payload.get("online"):
                    return True
        except Exception:
            pass
        time.sleep(0.25)
    return False


def emit(server_url: str, run_id: str, node_id: str, tag: str, payload: dict):
    requests.post(
        f"{server_url}/api/runs/{run_id}/messages",
        json={
            "from": node_id,
            "tag": tag,
            "payload": payload,
            "timestamp": int(time.time()),
        },
        timeout=5,
    ).raise_for_status()


def slug(value: str) -> str:
    normalized = re.sub(r"[^a-zA-Z0-9._-]+", "-", value.strip())
    normalized = normalized.strip("-")
    return normalized or "stream"


def extract_bytes(value) -> bytes:
    py = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(py, bytes):
        return py
    if isinstance(py, bytearray):
        return bytes(py)
    if isinstance(py, memoryview):
        return py.tobytes()
    if isinstance(py, list) and all(isinstance(item, int) for item in py):
        return bytes(py)
    if hasattr(value, "to_pylist"):
        pylist = value.to_pylist()
        if len(pylist) == 1 and isinstance(pylist[0], bytes):
            return pylist[0]
        if pylist and all(isinstance(item, int) for item in pylist):
            return bytes(pylist)
    raise ValueError(f"unsupported payload type: {type(py).__name__}")


def build_ffmpeg_command(
    ffmpeg_path: str,
    publish_url: str,
    fps: int,
    input_codec: str,
) -> list[str]:
    return [
        ffmpeg_path,
        "-hide_banner",
        "-loglevel",
        "warning",
        "-fflags",
        "nobuffer",
        "-flags",
        "low_delay",
        "-f",
        "image2pipe",
        "-framerate",
        str(fps),
        "-vcodec",
        input_codec,
        "-i",
        "-",
        "-an",
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-tune",
        "zerolatency",
        "-pix_fmt",
        "yuv420p",
        "-muxdelay",
        "0.1",
        "-rtsp_transport",
        "tcp",
        "-f",
        "rtsp",
        publish_url,
    ]


def detect_input_codec(metadata: dict) -> str:
    content_type = (metadata or {}).get("content_type", "").lower()
    if content_type == "image/jpeg":
        return "mjpeg"
    return "png"


def terminate(child: Optional[subprocess.Popen]):
    if child is None or child.poll() is not None:
        return
    child.terminate()
    try:
        child.wait(timeout=5)
    except subprocess.TimeoutExpired:
        child.kill()


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-stream-publish")
    run_id = env_str("DM_RUN_ID")
    server_url = env_str("DM_SERVER_URL", "http://127.0.0.1:3210")
    ffmpeg_path = env_str("FFMPEG_PATH", "ffmpeg")
    label = env_str("LABEL", "Live Stream")
    stream_name = env_str("STREAM_NAME", "main")
    fps = env_int("FPS", 5)
    width = env_int("WIDTH", 1280)
    height = env_int("HEIGHT", 720)
    codec = env_str("CODEC", "h264")
    print(
        f"[dm-stream-publish] init node_id={node_id} has_dora_config={bool(env_str('DORA_NODE_CONFIG'))}",
        flush=True,
    )
    node = create_node(node_id)
    print(
        f"[dm-stream-publish] start node_id={node_id} stream_name={stream_name} fps={fps} ffmpeg={ffmpeg_path}",
        flush=True,
    )

    media = get_media_status(server_url)
    if media.get("status") != "ready":
        raise RuntimeError(f"Media backend not ready: {media.get('status')}")

    path = f"run-{slug(run_id)}--{slug(node_id)}--{slug(stream_name)}"
    publish_url = f"rtsp://{media['host']}:{media['rtsp_port']}/{path}"
    stream_id = f"{node_id}/{stream_name}"

    child: Optional[subprocess.Popen] = None
    published = False

    try:
        while RUNNING:
            event = node.next(timeout=0.2)
            if event is None:
                continue
            if event["type"] == "STOP":
                return
            if event["type"] != "INPUT" or event["id"] != "frame":
                continue

            metadata = event.get("metadata") or {}
            blob = extract_bytes(event["value"])

            if child is None:
                input_codec = detect_input_codec(metadata)
                command = build_ffmpeg_command(ffmpeg_path, publish_url, fps, input_codec)
                child = subprocess.Popen(
                    command,
                    stdin=subprocess.PIPE,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.PIPE,
                    text=False,
                )

            if child.poll() is not None:
                stderr = b""
                if child.stderr is not None:
                    try:
                        stderr = child.stderr.read() or b""
                    except Exception:
                        stderr = b""
                raise RuntimeError(
                    f"ffmpeg exited with code {child.returncode}: {stderr.decode(errors='ignore').strip()}"
                )

            if child.stdin is None:
                raise RuntimeError("ffmpeg stdin is not available")

            child.stdin.write(blob)
            child.stdin.flush()

            if not published:
                if not wait_for_mediamtx_path(media, path):
                    print(
                        f"[dm-stream-publish] MediaMTX path not ready yet for {path}, delaying stream registration",
                        flush=True,
                    )
                    continue
                width_value = int(metadata.get("width", width) or width)
                height_value = int(metadata.get("height", height) or height)
                payload = {
                    "kind": "video",
                    "stream_id": stream_id,
                    "label": label,
                    "path": path,
                    "live": True,
                    "codec": codec,
                    "width": width_value,
                    "height": height_value,
                    "fps": fps,
                    "transport": {
                        "publish": "rtsp",
                        "play": ["webrtc", "hls"]
                    }
                }
                emit(server_url, run_id, node_id, "stream", payload)
                node.send_output("stream_id", pa.array([stream_id]))
                node.send_output(
                    "meta",
                    pa.array([json.dumps(payload)]),
                    {"content_type": "application/json"},
                )
                print(
                    f"[dm-stream-publish] stream registered path={path} stream_id={stream_id}",
                    flush=True,
                )
                published = True
    finally:
        terminate(child)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[dm-stream-publish] {exc}", file=sys.stderr, flush=True)
        raise
