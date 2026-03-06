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


def capture_png() -> bytes:
    with tempfile.TemporaryDirectory(prefix="dm_test_media_capture_") as tmp_dir:
        output_path = pathlib.Path(tmp_dir) / "capture.png"
        cmd = ["screencapture", "-x", "-t", "png", str(output_path)]
        print(f"dm-test-media-capture running: {' '.join(cmd)}", flush=True)
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.stdout.strip():
            print(result.stdout.strip(), flush=True)
        if result.returncode != 0:
            if result.stderr.strip():
                print(result.stderr.strip(), flush=True)
            raise SystemExit(
                "screencapture failed. Check macOS screen capture permission and active display access."
            )
        return output_path.read_bytes()


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


def main():
    mode = os.getenv("MODE", "screenshot").strip().lower()
    output_format = os.getenv("OUTPUT_FORMAT", "png").strip().lower()
    interval_sec = env_int("INTERVAL_SEC", 3)

    if mode not in {"screenshot", "repeat_screenshot"}:
        raise SystemExit(
            f"Unsupported MODE={mode}. Supported: screenshot, repeat_screenshot."
        )
    if output_format != "png":
        raise SystemExit(
            f"Unsupported OUTPUT_FORMAT={output_format}. Only png is implemented."
        )
    if sys.platform != "darwin":
        raise SystemExit("dm-test-media-capture currently supports macOS only.")

    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node = Node()
    if mode == "screenshot":
        emit_capture(node, mode, output_format)
        return

    while RUNNING:
        emit_capture(node, mode, output_format)
        for _ in range(max(1, interval_sec * 10)):
            if not RUNNING:
                break
            time.sleep(0.1)


if __name__ == "__main__":
    main()
