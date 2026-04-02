import os
import re
import sys
import time
from pathlib import Path

import pyarrow as pa
from dora import Node


MIME_TO_EXT = {
    "image/png": "png",
    "image/jpeg": "jpg",
    "audio/wav": "wav",
    "audio/x-wav": "wav",
    "audio/mpeg": "mp3",
    "video/mp4": "mp4",
    "application/json": "json",
    "text/plain": "txt",
}


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


def env_int(name: str) -> int | None:
    raw = env_str(name)
    if not raw:
        return None
    return int(raw)


def parse_size(raw: str | None) -> int | None:
    if raw is None or not raw.strip():
        return None
    match = re.fullmatch(r"\s*(\d+(?:\.\d+)?)\s*([kmgt]?b)?\s*", raw.strip(), re.I)
    if not match:
        raise ValueError(f"invalid size: {raw}")
    value = float(match.group(1))
    unit = (match.group(2) or "b").lower()
    factors = {
        "b": 1,
        "kb": 1024,
        "mb": 1024**2,
        "gb": 1024**3,
        "tb": 1024**4,
    }
    return int(value * factors[unit])


def parse_duration(raw: str | None) -> int | None:
    if raw is None or not raw.strip():
        return None
    match = re.fullmatch(r"\s*(\d+(?:\.\d+)?)\s*([smhd])\s*", raw.strip(), re.I)
    if not match:
        raise ValueError(f"invalid duration: {raw}")
    value = float(match.group(1))
    unit = match.group(2).lower()
    factors = {
        "s": 1,
        "m": 60,
        "h": 3600,
        "d": 86400,
    }
    return int(value * factors[unit])


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


def resolve_extension(configured: str, metadata: dict | None) -> str:
    if configured != "auto":
        return configured.lstrip(".")

    metadata = metadata or {}
    content_type = metadata.get("dm_type") or metadata.get("content_type") or ""
    return MIME_TO_EXT.get(content_type.lower(), "bin")


def list_files(directory: Path) -> list[Path]:
    return sorted(
        [path for path in directory.iterdir() if path.is_file()],
        key=lambda path: path.stat().st_mtime,
    )


def cleanup(directory: Path, max_age: int | None, max_files: int | None, max_total_size: int | None):
    files = list_files(directory)
    now = time.time()

    if max_age is not None:
        kept = []
        for path in files:
            age = now - path.stat().st_mtime
            if age > max_age:
                path.unlink(missing_ok=True)
            else:
                kept.append(path)
        files = kept

    if max_files is not None and len(files) > max_files:
        for path in files[: len(files) - max_files]:
            path.unlink(missing_ok=True)
        files = files[len(files) - max_files :]

    if max_total_size is not None:
        total = sum(path.stat().st_size for path in files)
        while files and total > max_total_size:
            oldest = files.pop(0)
            total -= oldest.stat().st_size
            oldest.unlink(missing_ok=True)


def main():
    node = Node()
    node_id = env_str("DM_NODE_ID", "dm-save")

    run_out_dir = env_str("DM_RUN_OUT_DIR")
    if not run_out_dir:
        raise SystemExit("DM_RUN_OUT_DIR is required")

    relative_dir = env_str("DIR")
    if not relative_dir:
        raise SystemExit("DIR config is required")

    output_dir = (Path(run_out_dir) / relative_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    naming = env_str("NAMING", "{timestamp}_{seq}")
    extension_cfg = env_str("EXTENSION", "auto")
    max_files = env_int("MAX_FILES")
    max_total_size = parse_size(os.getenv("MAX_TOTAL_SIZE"))
    max_age = parse_duration(os.getenv("MAX_AGE"))
    overwrite_latest = env_bool("OVERWRITE_LATEST", False)

    seq = 0
    stable_stem = re.sub(r"[^a-zA-Z0-9._-]+", "_", node_id)

    for event in node:
        if event["type"] != "INPUT" or event["id"] != "data":
            continue

        try:
            blob = extract_bytes(event["value"])
        except Exception as exc:
            print(f"[dm-save] Rejected: cannot extract bytes: {exc}", file=sys.stderr, flush=True)
            continue

        metadata = event.get("metadata") or {}
        extension = resolve_extension(extension_cfg, metadata)
        seq += 1

        if overwrite_latest:
            filename = f"{stable_stem}.{extension}"
        else:
            timestamp = time.strftime("%Y%m%d_%H%M%S")
            stem = naming.format(
                timestamp=timestamp,
                seq=f"{seq:04d}",
                node_id=node_id,
            )
            filename = f"{stem}.{extension}"

        output_path = output_dir / filename
        output_path.write_bytes(blob)

        cleanup(output_dir, max_age=max_age, max_files=max_files, max_total_size=max_total_size)

        node.send_output("path", pa.array([str(output_path)]))
        print(f"[DM-IO] SAVE {extension} -> {output_path} ({len(blob)} bytes)", flush=True)


if __name__ == "__main__":
    main()
