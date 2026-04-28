# DM Service Mental Model

> Status: working note, reflected by the current `codex/service` branch.

## One Sentence

Service is structurally like Node, but behaviorally like an MCP tool or a cloud
function.

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

## Runtime Ownership

Core owns service discovery, workspace management, manifest parsing, and generic
command-service invocation. Core can run a local command service because the
contract is process-local and self-contained: write one JSON request to stdin
and read one JSON result from stdout.

dm-server owns server-backed built-in services. These services need server
state, broadcast channels, runtime context, or existing server subsystems. For
example, `message.send` needs the run-scoped message store and the server's
message notification channel, so it is invoked by dm-server rather than by
core.

This split keeps core reusable while still allowing dm-server to expose common
platform capabilities as services.

## Current Runtime Semantics

Command services receive:

```json
{
  "method": "name",
  "input": {},
  "context": null
}
```

Command services return JSON on stdout. Stderr is diagnostic output and is
included in failure details when the command exits with a non-zero status. A
command that does not finish before its timeout fails with a structured timeout
error.

Server built-in services receive the same logical request, but execute inside
dm-server. Run-scoped built-ins use `context.run_id` as the first stable context
field.

dm-server also exposes a run-scoped invocation shortcut:

```text
POST /api/runs/{run_id}/services/{service_id}/invoke
```

This route injects `context.run_id` before dispatching the service call. It is
the preferred shape for future node SDK calls and run-detail Web surfaces,
because callers already know the run from the URL or runtime environment.

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
