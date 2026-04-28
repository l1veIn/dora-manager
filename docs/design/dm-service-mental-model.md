# DM Service Mental Model

> Status: working note, reflected by the current `codex/service` branch.

## One Sentence

Service is structurally like Node, but behaviorally like a Python cloud
function or an MCP tool.

## Structural Model

Service follows the same asset-management shape as Node. A service owns a
workspace directory, a manifest, source files, documentation, optional assets,
and optional install artifacts. Built-in services live under `services/<id>/`,
mirroring built-in nodes under `nodes/<id>/`.

This means the expected management surface is intentionally familiar:

- `service.json` is the service manifest, similar in role to a node `dm.json`.
- `README.md`, avatar files, scripts, dependency files, and binaries can live in
  the service workspace.
- CLI and Web should support list, describe, create, import, install,
  uninstall, open, inspect files, edit config, and read docs.
- The Web service detail page should keep the same broad shape as the node
  detail page, with an extra invocation-oriented view.

The goal is not to share every implementation detail with Node. The goal is to
avoid making users learn a second way to manage local dm assets.

## Behavioral Model

Service does not behave like Node. A Node is a long-lived stream participant in
a Dora dataflow. A Service is a structured operation that can be discovered,
described, invoked, and completed.

The behavioral contract is:

```text
service.method(input, context) -> output | structured_error
```

The current v0 protocol uses JSON input and JSON output. Method manifests
declare input and output schemas. Invocation failures should return stable
error codes, readable messages, and optional details.

This is intentionally closer to MCP tools, cloud functions, n8n actions, Dify
tools, and OpenAI tool calls than to Dora topics or Arrow streams.

## Python Runner

Service v0 is intentionally not a general runtime platform. A user service is a
Python workspace with a `service.py` entry script by default. Core owns service
discovery, workspace management, manifest parsing, dependency installation, and
Python invocation.

The minimal workspace is:

```text
services/<id>/
  service.json
  service.py
  README.md
```

This keeps Service close to a cloud-function authoring model: users write a
small Python entry point, while dm handles invocation, timeout, schema
validation, and diagnostics.

If a workload needs low-latency continuous processing, it should be a Node in a
Dora graph instead of a Service.

## Current Invocation Semantics

Python services receive:

```json
{
  "method": "name",
  "input": {},
  "context": null
}
```

Python services return JSON on stdout. Stderr is diagnostic output and is
included in failure details when the process exits with a non-zero status. A
service that does not finish before its timeout fails with a structured timeout
error.

For compatibility, manifests that still provide `runtime.exec` can be invoked
through the same JSON protocol. That path is legacy; new services should use
`entry`, defaulting to `service.py`.

## DM Platform APIs

dm-server platform capabilities such as message, config, run, artifact, and
media are not Python services. They are server APIs exposed over HTTP and, where
latency or graph integration needs it, Unix sockets. The existing `dm-bridge`
virtual node is the reference shape for this local IPC model.

`message` belongs here. It should not be implemented by a Python service that
calls back into dm-server, because that would create a recursive server ->
service -> server loop.

dm-server also exposes a run-scoped invocation shortcut:

```text
POST /api/runs/{run_id}/services/{service_id}/invoke
```

This route currently acts as a transitional server API for run-scoped platform
capabilities. Future SDKs should prefer direct HTTP or Unix socket access to
the platform API when the target is dm-server itself.

## Boundary With Node

Use Node when the capability is a long-running part of a dataflow, consumes or
produces high-throughput streams, or needs Dora topic semantics.

Use Service when the capability is a discrete action or query, such as sending
a message, reading config, asking for run status, calling a model API, reading
an artifact, or running one bounded transformation.

The practical rule is:

```text
Node is for streaming computation. Service is for structured calls.
```
