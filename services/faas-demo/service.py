#!/usr/bin/env python3
import json, sys

for line in sys.stdin:
    req = json.loads(line)
    payload = req.get("input", {})
    payload["result"] = "ok"
    print(json.dumps(payload), flush=True)
