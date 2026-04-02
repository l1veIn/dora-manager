import csv
import io
import json
import os
import sys
from datetime import datetime
from pathlib import Path

import pyarrow as pa
from dora import Node
from loguru import logger


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_bool(name: str, default: bool = False) -> bool:
    raw = env_str(name)
    if not raw:
        return default
    return raw.lower() in {"1", "true", "yes", "on"}


def serialize_csv(value) -> str:
    py = value.as_py() if hasattr(value, "as_py") else value
    output = io.StringIO()
    writer = csv.writer(output)
    if isinstance(py, dict):
        writer.writerow(py.keys())
        writer.writerow(py.values())
    elif isinstance(py, (list, tuple)):
        writer.writerow(py)
    else:
        writer.writerow([py])
    return output.getvalue().strip()


def serialize_value(value, fmt: str) -> str:
    if fmt == "text":
        py = value.as_py() if hasattr(value, "as_py") else value
        if isinstance(py, bytes):
            return py.decode("utf-8")
        return str(py)
    if fmt == "json":
        py = value.as_py() if hasattr(value, "as_py") else value
        return json.dumps(py, ensure_ascii=False)
    if fmt == "csv":
        return serialize_csv(value)
    raise ValueError(f"Unknown format: {fmt}")


def main():
    node = Node()
    node_id = env_str("DM_NODE_ID", "dm-log")
    run_out_dir = env_str("DM_RUN_OUT_DIR")
    relative_path = env_str("PATH")
    if not run_out_dir or not relative_path:
        raise SystemExit("DM_RUN_OUT_DIR and PATH are required")

    output_path = Path(run_out_dir) / relative_path
    if output_path.suffix == "":
        output_path = output_path / f"{node_id}.log"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    fmt = env_str("FORMAT", "text")
    add_timestamp = env_bool("TIMESTAMP", True)

    logger.remove()
    logger.add(
        output_path,
        format="{message}",
        rotation=env_str("ROTATION") or None,
        retention=env_str("RETENTION") or None,
        compression=env_str("COMPRESSION") or None,
    )

    emitted_path = False

    for event in node:
        if event["type"] != "INPUT" or event["id"] != "data":
            continue

        try:
            line = serialize_value(event["value"], fmt)
        except Exception as exc:
            print(f"[dm-log] Rejected: cannot serialize as {fmt}: {exc}", file=sys.stderr, flush=True)
            continue

        if add_timestamp:
            line = f"{datetime.now().isoformat()} | {line}"

        logger.info(line)
        if not emitted_path:
            node.send_output("path", pa.array([str(output_path)]))
            emitted_path = True
        print(f"[DM-IO] LOG {fmt} -> {output_path}", flush=True)


if __name__ == "__main__":
    main()
