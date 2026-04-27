# DM Service v0 Roadmap

> Status: wishlist and direction-setting note  
> Depends on: [service-design-philosophy.md](../service-design-philosophy.md)

## Purpose

This note freezes the current Service discussion as a product and engineering
wishlist, not as a fixed implementation plan. The intent is to keep the project
pointed in the right direction while leaving room to revise details once real
code and usage expose better paths.

Service is expected to coexist with Node, Dataflow, Run, Bridge, Message, and
Capability Binding. It is not a compatibility wrapper for the current
interaction system. It is a separate operation layer for capabilities that are
better expressed as discrete calls than as continuous Arrow streams.

## Product Shape

Service should feel familiar because the same shape appears in many mature
systems:

- OpenAI function/tool calling
- MCP tools
- Dify tools and workflow nodes
- n8n nodes
- Zapier actions
- serverless functions
- ROS services and actions
- VS Code commands

The repeated pattern is simple: a capability is discoverable, described by
schema, invokable with structured input, and returns structured output or a
task/event stream.

For dm, the useful phrase is:

```text
dm Service = local-first, run-aware, node-callable operation.
```

## North Star

The desired user-facing model is:

```python
dm.service("message").send({"tag": "text", "payload": {"content": "hello"}})
dm.service("config").get({"key": "model"})
dm.service("yolo").detect({"image": "..."})
```

The caller should not know whether the service is built into dm-server, backed
by SQLite, forwarded to an HTTP endpoint, launched as a command, implemented by
a daemon, or imported from another tool ecosystem.

## Wishlist

### Invocation Experience

- List available services from CLI and Web.
- Describe a service, its methods, and its input/output schemas.
- Invoke a service from CLI with JSON input.
- Invoke a service from Web.
- Verify the first loop with a minimal `add(x, y)` command service before
  expanding into run-scoped services or SDK calls.
- Invoke a service from Node code through a small SDK.
- Support global calls that do not require a run.
- Support run-scoped calls that receive `run_id` context.
- Support node-scoped calls that receive `run_id` and `node_id` context.
- Hide service location and transport from the caller.

### Service Types

- dm-server built-in public services.
- Local command services.
- Local script services.
- HTTP-backed services.
- Long-running daemon services.
- Run-scoped services started with a run.
- Stateless on-demand utility services.
- External API wrapper services.
- Imported MCP tools, if the mapping proves practical.

### Candidate Built-In Services

- `message.send`
- `message.list`
- `message.snapshots`
- `config.get`
- `config.set`
- `log.read`
- `log.tail`
- `artifact.list`
- `artifact.read`
- `media.status`
- `stream.list`
- `run.status`
- `run.metrics`
- `registry.list`

These are candidates, not a promise that all of them belong in v0.

### Node-Side Expectations

- A node can discover its `run_id`.
- A node can discover its `node_id`.
- A node can call a service without hand-writing HTTP details.
- A node can write messages through the message service.
- A node can read config through the config service.
- A node can check whether a service is available.
- A node can handle service-not-ready responses.
- A node receives structured errors.
- Node-side code stays short enough that using a Service feels lighter than
  creating a new dataflow path.

### Web-Side Expectations

- Web can show service lists.
- Web can show method schemas.
- Web can manually call service methods for debugging.
- Web can render simple forms from schemas.
- Web can display call results.
- Web can display service status and health.
- Web can call run-scoped and global services.

### Service Definition

- A service has a `service.json` manifest.
- The repository can ship default service manifests under `services/<id>/`,
  mirroring the existing `nodes/<id>/` layout for builtin nodes.
- A service has an id, version, and description.
- A service has methods.
- Each method has a description.
- Each method has an input schema.
- Each method has an output schema.
- A service can declare whether it is global or run-scoped.
- A service can declare whether it is persistent or on-demand.
- A service can declare timeout behavior.
- A service can declare environment variables.
- A service can declare backend/runtime type.

### Lifecycle And Operations

- Install service.
- Uninstall service.
- Start service.
- Stop service.
- Restart service.
- Query service status.
- Query service health.
- Query service readiness.
- Read service logs.
- Clean up run-scoped services when a run stops.
- Load built-in services when dm-server starts.
- Return readable and structured service failures.

### Long Tasks

- A method can return a `task_id`.
- Query task status.
- Query task progress.
- Cancel task.
- Fetch task result.
- Fetch task logs.
- Let CLI wait for task completion.
- Let Web display task progress.
- Let Node start a task asynchronously.

### Data And Protocol

- Default to JSON input and JSON output.
- Support file paths as input.
- Support artifacts as output.
- Support base64 payloads for small binary input.
- Support streaming results when needed.
- Support event push when needed.
- Support timeout.
- Support retry policy later, if needed.
- Support structured error codes.
- Support call trace ids.

### Developer Experience

- Create a service template.
- Validate `service.json`.
- Run a local service during development.
- Test one method locally.
- Generate or expose documentation.
- Generate Python client helpers.
- Generate TypeScript client helpers.
- Provide example services such as `echo`, `calculator`, `message`, and
  `yolo-detect`.
- Make service debug logs clear.

### Migration And Coexistence

- Keep existing message endpoints initially.
- Keep existing bridge and capability-binding paths initially.
- Add Service endpoints alongside existing endpoints.
- Wrap existing message behavior as a Service instead of deleting it first.
- Move a feature toward Service only when Service proves simpler in real usage.
- Keep high-throughput stream workloads on Node/Dataflow/Topic.

## Existing Patterns To Reuse

Service is not a new category of product problem. The v0 design should borrow
from existing systems instead of inventing names unnecessarily.

MCP is the closest external standard to track. MCP already has a mature model
for tool discovery, tool descriptions, JSON schemas, and tool invocation. dm
should not blindly become an MCP server, but `service.json` should stay easy to
map to MCP tools.

The likely useful direction is:

```text
dm Service ~= run-aware MCP tool
```

This keeps the door open for two future capabilities:

- expose dm services as MCP tools;
- import MCP tools as dm services.

## Backend Reuse Principle

Even if dm owns the Service abstraction, individual service backends should
reuse mature components where possible.

For the first message service, the current SQLite shape is still a good default:

```text
messages           append-only history
message_snapshots  latest value per node/tag
```

This behaves like a small local event store and preserves the zero-dependency
local-first product path. The design should avoid hard-coding SQLite so tightly
that later backends become impossible. Possible future backends include NATS,
Redis Streams, MQTT, EventStoreDB, or another local embedded queue, but none of
them should be introduced before there is a real need.

## First Implementation Bias

The first implementation should prove the shape without trying to finish the
platform.

Prefer:

- one or two built-in services;
- list/describe before broad invoke coverage;
- one global example and one run-scoped example;
- compatibility with current endpoints;
- thin SDK wrappers;
- stable call semantics.

Avoid in v0:

- replacing Bridge;
- deleting current Message endpoints;
- designing a full workflow engine;
- introducing a mandatory external message broker;
- forcing every node capability into Service;
- making SDKs carry business logic.

## Open Questions

- Should v0 service manifests use `methods` or `provides`?
- Should action-like long tasks be represented as a method type or as a naming
  convention around `start/status/cancel/result`?
- Should built-in services have physical `service.json` files or be generated
  from Rust definitions?
- Should Node SDK first use dm-server HTTP, local socket, or both?
- How much of service lifecycle belongs in dm-core before it becomes too broad?
- What is the smallest message service wrapper that proves the model without
  disrupting current interaction work?
