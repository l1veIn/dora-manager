# Message Service

The Message platform API exposes dm's run-scoped message store.

Message is hosted by dm-server because it needs access to the run message
database and the server's message notification channel. It should be used via
HTTP or Unix socket APIs, following the same local IPC direction as the
`dm-bridge` virtual node.

Run-scoped HTTP callers can use `/api/runs/{run_id}/services/message/invoke` to
have dm-server inject `context.run_id` automatically.

## Methods

- `send` appends a message to a run-scoped message store.
- `list` returns message history with optional filters.
- `snapshots` returns the latest message per `(node, tag)` pair.

## Example

```json
{
  "method": "send",
  "context": {"run_id": "run-123"},
  "input": {
    "from": "web",
    "tag": "text",
    "payload": {"content": "hello"}
  }
}
```
