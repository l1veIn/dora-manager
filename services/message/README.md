# Message Service

The Message service exposes dm's run-scoped message store as a server-backed
builtin service.

This service is hosted by dm-server because it needs access to the run message
database and the server's message notification channel. It uses the same
invocation shape as command services, but requires `context.run_id`.

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
