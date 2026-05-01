from __future__ import annotations

import inspect
import os
from pathlib import Path


def env_or_default(key: str, default: str | None = None) -> str | None:
    value = os.environ.get(key)
    if value is None or value == "":
        return default
    return value


def detect_caller_id() -> str:
    sdk_dir = Path(__file__).resolve().parent
    for frame in inspect.stack()[1:]:
        filename = frame.filename
        if not filename or filename.startswith("<"):
            continue
        path = Path(filename).resolve()
        if sdk_dir not in path.parents and path != Path(__file__).resolve():
            return path.name
    return "python"


def normalize_url(url: str) -> str:
    return url.rstrip("/")
