#!/usr/bin/env python3
"""Reverse text function for dm-server FaaS."""
import json, sys

for line in sys.stdin:
    req = json.loads(line)
    payload = req.get("input", {})
    text = payload.get("text", "")
    reversed_text = text[::-1]
    result = {
        "original": text,
        "reversed": reversed_text,
        "length": len(text),
    }
    print(json.dumps(result), flush=True)
