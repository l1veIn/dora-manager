#!/usr/bin/env python3
import json
import sys


def as_number(value, name):
    if not isinstance(value, (int, float)):
        raise ValueError(f"{name} must be a number")
    return value


def main():
    request = json.load(sys.stdin)
    method = request.get("method")
    payload = request.get("input") or {}

    if method != "add":
        raise ValueError(f"unsupported method: {method}")

    x = as_number(payload.get("x"), "x")
    y = as_number(payload.get("y"), "y")
    json.dump({"result": x + y}, sys.stdout)


if __name__ == "__main__":
    main()
