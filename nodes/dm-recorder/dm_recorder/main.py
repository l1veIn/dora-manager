import os
import struct
import sys
import wave
from pathlib import Path

import pyarrow as pa
from dora import Node


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_int(name: str, default: int) -> int:
    raw = env_str(name)
    if not raw:
        return default
    return int(raw)


def extract_pcm_bytes(value) -> bytes:
    py = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(py, bytes):
        return py
    if isinstance(py, bytearray):
        return bytes(py)
    if isinstance(py, list) and py and all(isinstance(item, (int, float)) for item in py):
        return struct.pack(f"<{len(py)}f", *[float(item) for item in py])
    raise ValueError(f"unsupported audio payload type: {type(py).__name__}")


def main():
    node = Node()
    run_out_dir = env_str("DM_RUN_OUT_DIR")
    relative_path = env_str("PATH", "recording.wav")
    output_path = Path(run_out_dir) / relative_path
    output_path.parent.mkdir(parents=True, exist_ok=True)

    wav = wave.open(str(output_path), "wb")
    wav.setnchannels(env_int("CHANNELS", 1))
    wav.setsampwidth(4)
    wav.setframerate(env_int("SAMPLE_RATE", 16000))

    emitted_path = False
    try:
        for event in node:
            if event["type"] == "STOP":
                break
            if event["type"] != "INPUT" or event["id"] != "data":
                continue

            try:
                chunk = extract_pcm_bytes(event["value"])
            except Exception as exc:
                print(f"[dm-recorder] Rejected: cannot serialize audio: {exc}", file=sys.stderr, flush=True)
                continue

            wav.writeframes(chunk)
            if not emitted_path:
                node.send_output("path", pa.array([str(output_path)]))
                emitted_path = True
            print(f"[DM-IO] RECORD wav -> {output_path} ({len(chunk)} bytes)", flush=True)
    finally:
        wav.close()


if __name__ == "__main__":
    main()
