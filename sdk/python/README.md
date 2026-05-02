# dm Python SDK

Lightweight Python client for dora-manager message and service APIs.

## Install

```bash
cd /path/to/dora-manager/sdk/python
pip install -e .
```

This installs the `dm` package with its dependencies (`requests`, `websockets`).

## Usage inside a dora run

When used inside a dora dataflow node, `DM_RUN_ID` is automatically set by the transpiler,
so `Message()` works without arguments:

```python
import dm

msg = dm.Message()  # run_id from DM_RUN_ID environment variable
msg.send("status", {"ready": True})
result = msg.get(tag="status")
```

## Manual testing with a real run

For testing outside a dora node, get a real `run_id` from `dm runs` or the Web UI:

```python
import dm

msg = dm.Message(run_id="<real-run-id>", server_url="http://127.0.0.1:3210")
seq = msg.send("text", {"content": "hello"}, from_="example.py")
messages = msg.get(after_seq=seq - 1)
print(messages[-1])
```

## Subscribe to real-time messages

```python
import dm

msg = dm.Message()
with msg.subscribe(tag="text") as stream:
    for event in stream:
        print(event["seq"], event["from"], event["payload"])
```

## Call services (FaaS)

Functions run as Python subprocesses managed by dm-server:

```python
import dm

svc = dm.Service()
result = svc.invoke("faas-demo", method="run", input={"text": "hello"})
print(result)
# {"original": "hello", "reversed": "olleh", "length": 5}
```

List available services:

```python
import dm

for service in dm.Service().list():
    print(service["id"], service.get("description", ""))
```

## Full end-to-end demo

See `demos/demo-sdk-interaction.yml` for a complete example:

1. Start the demo: `dm start demos/demo-sdk-interaction.yml`
2. Open the Web UI at http://127.0.0.1:3210
3. Go to Runs → select the active run → enter text in the input panel
4. The demo node reverses the text via FaaS and sends it back

## Environment variables

| Variable | Default | Used by |
|---|---|---|
| `DM_RUN_ID` | — | `Message()` when `run_id` omitted |
| `DM_SERVER_URL` | `http://127.0.0.1:3210` | `Message()` HTTP/WS |
