#!/usr/bin/env python3
import json
import sys


def as_number(value, name):
    if not isinstance(value, (int, float)):
        raise ValueError(f"{name} must be a number")
    return value


def main():
    for line in sys.stdin:
        if not line.strip():
            continue
        request = json.loads(line)
        method = request.get("method")
        payload = request.get("input") or {}

        if method not in ("run", "add"):
            raise ValueError(f"unsupported method: {method}")

        x = as_number(payload.get("x"), "x")
        y = as_number(payload.get("y"), "y")
        print(json.dumps({"result": x + y}), flush=True)


if __name__ == "__main__":
    main()
