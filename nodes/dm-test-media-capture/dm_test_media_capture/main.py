import json
import os
import pathlib
import signal
import subprocess
import sys
import tempfile
import time

import pyarrow as pa
from dora import Node

RUNNING = True


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def env_int(name: str, default: int) -> int:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return int(raw)


def run_capture_command(cmd: list[str], error_message: str):
    print(f"dm-test-media-capture running: {' '.join(cmd)}", flush=True)
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.stdout.strip():
        print(result.stdout.strip(), flush=True)
    if result.returncode != 0:
        if result.stderr.strip():
            print(result.stderr.strip(), flush=True)
        raise SystemExit(error_message)


def capture_png() -> bytes:
    with tempfile.TemporaryDirectory(prefix="dm_test_media_capture_") as tmp_dir:
        output_path = pathlib.Path(tmp_dir) / "capture.png"
        cmd = ["screencapture", "-x", "-t", "png", str(output_path)]
        run_capture_command(
            cmd,
            "screencapture failed. Check macOS screen capture permission and active display access.",
        )
        return output_path.read_bytes()


def capture_video(output_format: str, clip_duration_sec: int) -> tuple[bytes, str]:
    with tempfile.TemporaryDirectory(prefix="dm_test_media_capture_") as tmp_dir:
        tmp_dir_path = pathlib.Path(tmp_dir)
        mov_path = tmp_dir_path / "capture.mov"
        cmd = [
            "screencapture",
            "-x",
            "-v",
            "-V",
            str(clip_duration_sec),
            str(mov_path),
        ]
        run_capture_command(
            cmd,
            "video screencapture failed. Check macOS screen capture permission and active display access.",
        )

        if output_format == "mov":
            return mov_path.read_bytes(), "video/quicktime"

        if output_format != "mp4":
            raise SystemExit(
                f"Unsupported OUTPUT_FORMAT={output_format}. Supported: png, mov, mp4."
            )

        ffmpeg = os.getenv("FFMPEG_BIN", "ffmpeg")
        mp4_path = tmp_dir_path / "capture.mp4"
        convert_cmd = [
            ffmpeg,
            "-y",
            "-i",
            str(mov_path),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
            str(mp4_path),
        ]
        run_capture_command(
            convert_cmd,
            "ffmpeg conversion failed. Install ffmpeg or use OUTPUT_FORMAT=mov.",
        )
        return mp4_path.read_bytes(), "video/mp4"


def emit_capture(node: Node, mode: str, output_format: str):
    data = capture_png()
    metadata = {
        "content_type": "image/png",
        "mode": mode,
        "filename": "capture.png",
    }
    node.send_output("image", pa.array(data, type=pa.uint8()), metadata)
    node.send_output(
        "meta",
        pa.array(
            [
                json.dumps(
                    {
                        "mode": mode,
                        "content_type": "image/png",
                        "byte_size": len(data),
                        "format": output_format,
                        "captured_at": time.time(),
                    }
                )
            ]
        ),
        {"content_type": "application/json"},
    )


def emit_video(node: Node, mode: str, output_format: str, clip_duration_sec: int):
    data, content_type = capture_video(output_format, clip_duration_sec)
    extension = "mp4" if output_format == "mp4" else "mov"
    metadata = {
        "content_type": content_type,
        "mode": mode,
        "filename": f"capture.{extension}",
    }
    node.send_output("video", pa.array(data, type=pa.uint8()), metadata)
    node.send_output(
        "meta",
        pa.array(
            [
                json.dumps(
                    {
                        "mode": mode,
                        "content_type": content_type,
                        "byte_size": len(data),
                        "format": output_format,
                        "clip_duration_sec": clip_duration_sec,
                        "captured_at": time.time(),
                    }
                )
            ]
        ),
        {"content_type": "application/json"},
    )


def main():
    mode = os.getenv("MODE", "screenshot").strip().lower()
    output_format = os.getenv("OUTPUT_FORMAT", "png").strip().lower()
    interval_sec = env_int("INTERVAL_SEC", 3)
    clip_duration_sec = env_int("CLIP_DURATION_SEC", 5)

    if mode not in {"screenshot", "repeat_screenshot", "record_clip"}:
        raise SystemExit(
            f"Unsupported MODE={mode}. Supported: screenshot, repeat_screenshot, record_clip."
        )
    if mode in {"screenshot", "repeat_screenshot"} and output_format != "png":
        raise SystemExit(
            f"Unsupported OUTPUT_FORMAT={output_format}. Only png is implemented."
        )
    if mode == "record_clip" and output_format not in {"mov", "mp4"}:
        raise SystemExit(
            f"Unsupported OUTPUT_FORMAT={output_format}. Supported for record_clip: mov, mp4."
        )
    if sys.platform != "darwin":
        raise SystemExit("dm-test-media-capture currently supports macOS only.")

    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node = Node()
    if mode == "screenshot":
        while RUNNING:
            event = node.next(timeout=0.05)
            if event is None:
                emit_capture(node, mode, output_format)
                return
            if event["type"] == "INPUT":
                emit_capture(node, mode, output_format)
            elif event["type"] == "STOP":
                return
    elif mode == "record_clip":
        while RUNNING:
            event = node.next()
            if event is None or event["type"] == "STOP":
                return
            if event["type"] == "INPUT":
                emit_video(node, mode, output_format, clip_duration_sec)

    while RUNNING:
        emit_capture(node, mode, output_format)
        for _ in range(max(1, interval_sec * 10)):
            if not RUNNING:
                break
            time.sleep(0.1)


if __name__ == "__main__":
    main()
