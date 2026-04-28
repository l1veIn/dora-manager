#!/usr/bin/env python3
import json
import sys

from message_db import MessageDB
from normalize import normalize_payload


db = MessageDB()


def error(code, message):
    return {"error": {"code": code, "message": message}}


def required_string(input_data, key, method):
    value = input_data.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"Service 'message.{method}' input requires '{key}'")
    return value


def optional_int(input_data, key, method):
    if key not in input_data:
        return None
    value = input_data.get(key)
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError(
            f"Service 'message.{method}' input field '{key}' must be an integer"
        )
    return value


def optional_limit(input_data):
    value = optional_int(input_data, "limit", "list")
    if value is not None and value < 0:
        raise ValueError(
            "Service 'message.list' input field 'limit' must be a non-negative integer"
        )
    return value


def optional_string_array(input_data, key, method):
    if key not in input_data:
        return None
    value = input_data.get(key)
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise ValueError(
            f"Service 'message.{method}' input field '{key}' must be an array of strings"
        )
    return value


def dispatch(method, run_id, input_data):
    if method == "send":
        node_id = required_string(input_data, "from", "send")
        tag = required_string(input_data, "tag", "send")
        if "payload" not in input_data:
            raise ValueError("Service 'message.send' input requires 'payload'")
        timestamp = optional_int(input_data, "timestamp", "send")
        payload = normalize_payload(tag, input_data["payload"])
        seq = db.push(run_id, node_id, tag, payload, timestamp)
        return {"seq": seq}

    if method == "list":
        desc = input_data.get("desc", False)
        if not isinstance(desc, bool):
            raise ValueError("Service 'message.list' input field 'desc' must be a boolean")
        return db.list(
            run_id,
            after_seq=optional_int(input_data, "after_seq", "list"),
            before_seq=optional_int(input_data, "before_seq", "list"),
            from_filter=optional_string_array(input_data, "from", "list"),
            tag=optional_string_array(input_data, "tag", "list"),
            limit=optional_limit(input_data),
            desc=desc,
        )

    if method == "snapshots":
        return {"snapshots": db.snapshots(run_id)}

    return error(
        "method_not_found", f"Service 'message' does not declare method '{method}'"
    )


def handle_request(request):
    method = request.get("method")
    input_data = request.get("input") or {}
    context = request.get("context") or {}
    run_id = context.get("run_id")
    if not isinstance(run_id, str) or not run_id.strip():
        return error("context_required", "message service requires context.run_id")
    if not isinstance(input_data, dict):
        return error("input_validation_failed", "message service input must be an object")

    try:
        return dispatch(method, run_id, input_data)
    except ValueError as exc:
        return error("input_validation_failed", str(exc))
    except Exception as exc:
        return error("internal", str(exc))


def main():
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            request = json.loads(line)
            result = handle_request(request)
        except json.JSONDecodeError as exc:
            result = error("invalid_json", str(exc))
        sys.stdout.write(json.dumps(result, ensure_ascii=False, separators=(",", ":")) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
