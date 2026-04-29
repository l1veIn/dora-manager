# dm Python SDK

Lightweight Python client for dora-manager message and service APIs.

## Install for local development

```bash
cd ~/Desktop/dora-manager/sdk/python
python -m pip install -e .
```

## Send and pull messages

```python
import dm

msg = dm.Message(run_id="test-e2e", server_url="http://127.0.0.1:3210")
seq = msg.send("text", {"content": "hello"}, from_="example.py")
messages = msg.pull(after_seq=seq - 1)
print(messages[-1])
```

Inside a dora run, `DM_RUN_ID` and `DM_SERVER_URL` can provide the defaults:

```python
import dm

msg = dm.Message()
msg.send("status", {"ready": True})
```

## Call services

```python
import dm

svc = dm.Service(faasd_url="http://127.0.0.1:5001")
result = svc.invoke("add", method="run", input={"x": 7, "y": 11})
print(result)
```

List available services:

```python
import dm

for service in dm.Service().list():
    print(service["id"], service.get("description", ""))
```

## Environment variables

- `DM_RUN_ID`: current run id, required by `Message()` when `run_id` is omitted.
- `DM_SERVER_URL`: dm-server URL, default `http://127.0.0.1:3210`.
- `DM_FAASD_URL`: dm-faasd URL, default `http://127.0.0.1:5001`.
