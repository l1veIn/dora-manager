# Built-In Services

Built-in services live under `services/<id>/`. The directory structure mirrors
built-in nodes under `nodes/<id>/`: each service owns a workspace with a
`service.json` manifest, documentation, and any supporting files it needs.

The structural model is Node-like, but the runtime model is call-oriented. A
service method is invoked with JSON input and optional JSON context, then
returns JSON output or a structured error.

Current runtime support:

- `command`: implemented in core. dm starts the configured command in the
  service workspace, sends one JSON request on stdin, and reads one JSON result
  from stdout.
- `builtin`: hosted by dm-server when the service needs server state or existing
  server subsystems.

The first command example is `add`, which verifies the generic invocation loop.
The first server-backed builtin is `message`, which exposes run-scoped message
operations through the service invocation surface.

