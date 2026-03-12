"""dm-microphone: Microphone node with device selection support."""

import json
import os
import time

import numpy as np
import pyarrow as pa
import sounddevice as sd
from dora import Node


SAMPLE_RATE = int(os.getenv("SAMPLE_RATE", "16000"))
MAX_DURATION = float(os.getenv("MAX_DURATION", "0.1"))


def list_input_devices():
    """Return available input devices as JSON-serializable list."""
    devices = sd.query_devices()
    result = []
    default_input = sd.default.device[0]

    for idx, dev in enumerate(devices):
        if dev["max_input_channels"] <= 0:
            continue
        result.append({
            "id": str(idx),
            "name": dev["name"],
            "channels": dev["max_input_channels"],
            "sample_rate": int(dev["default_samplerate"]),
            "default": idx == default_input,
        })
    return result


def main():
    """Main entry point."""
    node = Node()

    # Publish device list on startup
    devices = list_input_devices()
    node.send_output(
        "devices",
        pa.array([json.dumps(devices)]),
        {"content_type": "application/json"},
    )

    # Determine initial device
    current_device = "default"  # Matches panel default
    sample_rate = SAMPLE_RATE
    enabled = False  # Start paused, wait for switch

    buffer = []
    start_recording_time = time.time()
    finished = False

    def callback(indata, frames, time_info, status):
        nonlocal buffer, start_recording_time, finished

        if time.time() - start_recording_time > MAX_DURATION:
            audio_data = np.array(buffer).ravel().astype(np.float32) / 32768.0
            node.send_output(
                "audio",
                pa.array(audio_data),
                {"sample_rate": sample_rate},
            )
            buffer = []
            start_recording_time = time.time()
        else:
            buffer.extend(indata[:, 0])

    def start_stream(device_id=None):
        """Start audio input stream with given device."""
        device = None
        if device_id is not None:
            try:
                device = int(device_id)
            except (ValueError, TypeError):
                device = None  # "default" or invalid → system default
        return sd.InputStream(
            device=device,
            callback=callback,
            dtype=np.int16,
            channels=1,
            samplerate=sample_rate,
        )

    stream = None  # Start without stream (mic disabled)

    while not finished:
        event = node.next(timeout=0.05)
        if event is None:
            finished = True
            break
        if event["type"] == "INPUT":
            input_id = event["id"]

            if input_id == "enabled":
                # Toggle microphone on/off
                raw = event["value"][0].as_py()
                new_enabled = str(raw).lower() in ("true", "1", "yes")
                if new_enabled and not enabled:
                    buffer = []
                    start_recording_time = time.time()
                    stream = start_stream(current_device)
                    stream.start()
                elif not new_enabled and enabled:
                    if stream is not None:
                        stream.stop()
                        stream.close()
                        stream = None
                        buffer = []
                enabled = new_enabled

            elif input_id == "device_id":
                # Switch microphone device
                new_device = event["value"][0].as_py()
                if str(new_device) != str(current_device):
                    current_device = new_device
                    if enabled and stream is not None:
                        stream.stop()
                        stream.close()
                        buffer = []
                        stream = start_stream(current_device)
                        stream.start()
                    # Re-publish device list with updated selection
                    devices = list_input_devices()
                    for d in devices:
                        d["default"] = d["id"] == str(current_device)
                    node.send_output(
                        "devices",
                        pa.array([json.dumps(devices)]),
                        {"content_type": "application/json"},
                    )

    if stream is not None:
        stream.stop()
        stream.close()

