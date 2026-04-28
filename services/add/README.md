# Add Service

Minimal Python service for verifying dm service invocation.

## Methods

- `add`: accepts `{"x": 1, "y": 2}` and returns `{"result": 3}`.

## Protocol

The service entry script reads one JSON request from stdin:

```json
{"method":"add","input":{"x":1,"y":2}}
```

It writes one JSON response to stdout.
