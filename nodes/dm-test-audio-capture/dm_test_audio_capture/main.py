import io
import json
import os
import signal
import sys
import time
import wave

import numpy as np
import pyarrow as pa
import sounddevice as sd
from dora import Node

RUNNING = True


def env_int(name: str, default: int) -> int:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return int(raw)


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def to_wav_bytes(samples: np.ndarray, sample_rate: int, channels: int) -> bytes:
    buf = io.BytesIO()
    with wave.open(buf, "wb") as wav:
        wav.setnchannels(channels)
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        wav.writeframes(samples.astype(np.int16).tobytes())
    return buf.getvalue()


def main():
    if sys.platform != "darwin":
        raise SystemExit("dm-test-audio-capture currently supports macOS only.")

    mode = os.getenv("MODE", "once").strip().lower()
    duration_sec = env_int("DURATION_SEC", 3)
    sample_rate = env_int("SAMPLE_RATE", 16000)
    channels = env_int("CHANNELS", 1)
    frame_count = duration_sec * sample_rate
    if mode not in {"once", "repeat"}:
        raise SystemExit("Unsupported MODE. Supported: once, repeat.")

    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node = Node()
    while RUNNING:
        print(
            f"dm-test-audio-capture recording {duration_sec}s at {sample_rate}Hz ({channels}ch)",
            flush=True,
        )
        recording = sd.rec(
            frame_count, samplerate=sample_rate, channels=channels, dtype="int16"
        )
        sd.wait()
        wav_bytes = to_wav_bytes(recording, sample_rate, channels)
        float_samples = (recording.reshape(-1).astype(np.float32)) / 32768.0

        node.send_output(
            "audio",
            pa.array(wav_bytes, type=pa.uint8()),
            {"content_type": "audio/wav", "filename": "capture.wav"},
        )
        node.send_output(
            "audio_stream",
            pa.array(float_samples, type=pa.float32()),
            {"sample_rate": sample_rate, "content_type": "application/x-audio-f32"},
        )
        node.send_output(
            "meta",
            pa.array(
                [
                    json.dumps(
                        {
                            "content_type": "audio/wav",
                            "duration_sec": duration_sec,
                            "sample_rate": sample_rate,
                            "channels": channels,
                            "byte_size": len(wav_bytes),
                            "mode": mode,
                            "captured_at": time.time(),
                        }
                    )
                ]
            ),
            {"content_type": "application/json"},
        )
        if mode == "once":
            break


if __name__ == "__main__":
    main()
