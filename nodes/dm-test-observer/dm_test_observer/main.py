import json
import os
import time

import pyarrow as pa
from dora import Node


def normalize_json(value):
    if isinstance(value, dict):
        return {str(k): normalize_json(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [normalize_json(v) for v in value]
    if isinstance(value, (str, int, float, bool)) or value is None:
        return value
    return str(value)


def stringify(value) -> str:
    try:
        if hasattr(value, "to_pylist"):
            return json.dumps(value.to_pylist())
    except Exception:
        pass
    try:
        return json.dumps(value.to_numpy().tolist())
    except Exception:
        return str(value)


def main():
    max_events = int(os.getenv("MAX_EVENTS", "1000"))
    node = Node()
    event_count = 0
    state = {}

    while True:
        event = node.next()
        if event is None:
            break
        if event["type"] != "INPUT":
            continue

        input_id = event["id"]
        event_count += 1
        state[input_id] = {
            "value": stringify(event["value"]),
            "metadata": normalize_json(dict(event.get("metadata", {}))),
            "seen_at": time.time(),
        }

        summary = {
            "event_count": event_count,
            "last_input": input_id,
            "known_inputs": sorted(state.keys()),
            "last_metadata": state[input_id]["metadata"],
        }
        text = (
            f"event={event_count} input={input_id} known={','.join(summary['known_inputs'])}"
        )
        print(text, flush=True)
        node.send_output("summary_text", pa.array([text]), {"content_type": "text/plain"})
        node.send_output(
            "summary_json",
            pa.array([json.dumps(summary)]),
            {"content_type": "application/json"},
        )

        if event_count >= max_events:
            break


if __name__ == "__main__":
    main()
