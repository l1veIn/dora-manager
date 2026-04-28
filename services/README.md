# Built-In Services

Built-in services live under `services/<id>/`. The directory structure mirrors
built-in nodes under `nodes/<id>/`: each service owns a workspace with a
`service.json` manifest, documentation, and any supporting files it needs.

The structural model is Node-like, but the runtime model is call-oriented. For
v0, user-authored services are Python function workspaces. A service method is
invoked with JSON input and optional JSON context, then returns JSON output or
a structured error.

Current service support:

- Python entry scripts: implemented in core. dm starts `service.py` or the
  configured `entry`, sends one JSON request on stdin, and reads one JSON
  result from stdout.
- Legacy `runtime.exec`: still accepted for compatibility while the v0 model
  converges on Python entry scripts.

The first Python example is `add`, which verifies the generic invocation loop.
dm-server platform APIs such as message should use HTTP or Unix socket
interfaces rather than Python services that call back into dm-server.
