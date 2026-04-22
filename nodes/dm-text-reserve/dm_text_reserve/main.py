import signal

import pyarrow as pa
from dora import Node


RUNNING = True


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def extract_text(value) -> str:
    if hasattr(value, "to_pylist"):
        pylist = value.to_pylist()
        if len(pylist) == 1:
            raw = pylist[0]
        else:
            raw = pylist
    else:
        raw = value.as_py() if hasattr(value, "as_py") else value

    if isinstance(raw, bytes):
        return raw.decode("utf-8")
    if isinstance(raw, list):
        return str(raw[0] if raw else "")
    return "" if raw is None else str(raw)


def main():
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node = Node()

    for event in node:
        if not RUNNING:
            break
        if event["type"] != "INPUT" or event["id"] != "text":
            continue

        content = extract_text(event["value"])
        node.send_output("text", pa.array([content[::-1]]))


if __name__ == "__main__":
    main()
