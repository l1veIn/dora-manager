import os
from pathlib import PurePosixPath


def normalize_payload(tag, payload):
    if tag == "input":
        return payload

    if tag == "stream":
        return normalize_stream_payload(payload)

    if isinstance(payload, dict) and isinstance(payload.get("file"), str):
        normalized = dict(payload)
        normalized["file"] = normalize_relative_path(payload["file"])
        return normalized

    return payload


def normalize_stream_payload(payload):
    if not isinstance(payload, dict):
        raise ValueError("Stream payload must be an object")

    path = payload.get("path")
    stream_id = payload.get("stream_id")
    kind = payload.get("kind")
    if not isinstance(path, str):
        raise ValueError("Stream payload requires 'path'")
    if not isinstance(stream_id, str):
        raise ValueError("Stream payload requires 'stream_id'")
    if not isinstance(kind, str):
        raise ValueError("Stream payload requires 'kind'")

    normalized = dict(payload)
    normalized["path"] = normalize_relative_path(path)
    normalized["stream_id"] = stream_id
    normalized["kind"] = kind
    if "live" not in normalized:
        normalized["live"] = True
    return normalized


def normalize_relative_path(path):
    if not isinstance(path, str):
        raise ValueError("Path must be a string")
    if os.path.isabs(path):
        raise ValueError("Expected path relative to run out dir")

    parts = []
    for part in PurePosixPath(path.replace("\\", "/")).parts:
        if part in ("", "."):
            continue
        if part == "..":
            raise ValueError("Invalid relative path")
        parts.append(part)

    text = "/".join(parts)
    if not text:
        raise ValueError("Path must not be empty")
    return text
