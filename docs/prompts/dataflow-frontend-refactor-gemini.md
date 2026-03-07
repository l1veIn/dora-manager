# DM Dataflow Frontend Refactor Guide

This document summarizes how the frontend should think about dataflows after
the latest backend refactor.

Do not use the old "single yml file" mental model.

## Core model

A dataflow is now a project directory under:

```text
~/.dm/dataflows/<name>/
  dataflow.yml
  config.json
  flow.json
  .history/
```

Meaning:

- `dataflow.yml`
  the actual Dora workflow source
- `config.json`
  flow-level config override
- `flow.json`
  business/display metadata
- `.history/*.yml`
  snapshots of previous `dataflow.yml` contents

The frontend should treat one dataflow as one project/workspace, not as one raw
text file.

## What matters in the UI

The two main pages can stay simple:

- a dataflow list page
- a dataflow detail page

The frontend does not need to invent a separate package model.

## Existing backend APIs

### List

`GET /api/dataflows`

Returns a list of dataflow projects. Each item includes:

- file info
- `meta`
- `executable`

The list response is already project-oriented.

### Detail

`GET /api/dataflows/{name}`

Returns:

- `name`
- `yaml`
- `meta`
- `executable`

This should be the primary source for the detail page.

### Save YAML

`POST /api/dataflows/{name}`

Request:

```json
{ "yaml": "..." }
```

Response returns the updated project:

- `name`
- `yaml`
- `meta`
- `executable`

Saving YAML automatically creates a `.history` snapshot when content changed.

### Flow metadata

- `GET /api/dataflows/{name}/meta`
- `POST /api/dataflows/{name}/meta`

Current `flow.json` shape:

```json
{
  "id": "qwen-dev",
  "name": "Qwen Dev",
  "description": "",
  "type": "",
  "tags": [],
  "author": null,
  "cover": null,
  "created_at": "2026-03-07T12:00:00Z",
  "updated_at": "2026-03-07T12:00:00Z"
}
```

### Flow config

- `GET /api/dataflows/{name}/config`
- `POST /api/dataflows/{name}/config`

Response shape:

```json
{
  "config": { ... },
  "executable": { ... }
}
```

### History

- `GET /api/dataflows/{name}/history`
- `GET /api/dataflows/{name}/history/{version}`
- `POST /api/dataflows/{name}/history/{version}/restore`

History only versions `dataflow.yml`.

### Import

`POST /api/dataflows/import`

Request:

```json
{
  "sources": [
    "/path/to/dataflow.yml",
    "/path/to/dataflow-project",
    "https://github.com/l1veIn/dora-manager/blob/master/tests/dataflows/system-test-full.yml"
  ]
}
```

Response:

```json
{
  "imported": [
    {
      "name": "system-test-full",
      "executable": { ... }
    }
  ],
  "failed": []
}
```

### Executable state

`GET /api/dataflows/{name}/inspect`

Also, executable state is already included in:

- list items
- detail response
- config response
- import response

## Executable state is important

Every dataflow has a derived executable state.

Current state values:

- `ready`
- `missing_nodes`
- `invalid_yaml`

Key fields:

- `can_run`
- `can_configure`
- `missing_nodes`
- `invalid_yaml`
- `error`

The frontend should use executable state in:

- list page
- detail page
- config page
- run action gating

Important:

Run is now hard-blocked by backend inspect before Dora starts.

So if the frontend sees `can_run == false`, it should present that clearly.

## Config behavior

The frontend should not invent a separate flow config schema inside `flow.json`.

Instead:

- read the current flow `config.json`
- inspect nodes used in `dataflow.yml`
- fetch node metadata/config schema as needed
- aggregate node config fields on the frontend
- save actual chosen values back into `/api/dataflows/{name}/config`

Current priority in backend runtime/transpile is:

1. inline node `config:` in `dataflow.yml`
2. dataflow `config.json`
3. node `config.json`
4. node `dm.json -> config_schema.default`

## What changed compared to old frontend assumptions

Old assumption:

- dataflow == one saved yaml file

New assumption:

- dataflow == one project
- yaml is only one part of the project
- metadata/config/history/import are all first-class

## Refactor direction

The frontend refactor should do these things:

1. Treat `/dataflows` as a project index, not a file browser
2. Treat `/dataflows/[name]` as a project workspace
3. Use the project response as the main detail payload
4. Show executable state prominently
5. Wire YAML save, metadata save, config save, history restore, and import
6. Stop assuming list/detail responses only contain raw YAML text

## Keep the UX simple

The backend model is more structured now, but the UI can still stay lean:

- list page
- detail page
- editable YAML
- metadata editor
- config editor
- history list
- import entry

That is enough for the current phase.
