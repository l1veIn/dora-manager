# Message Service

The Message service describes dm's run-scoped message store.

It is currently a manifest-level service: existing dm-server message endpoints
continue to provide the runtime behavior, while this service entry makes the
capability visible through the Service registry.

## Methods

- `send` appends a message to a run-scoped message store.
- `list` returns message history with optional filters.
- `snapshots` returns the latest message per `(node, tag)` pair.
