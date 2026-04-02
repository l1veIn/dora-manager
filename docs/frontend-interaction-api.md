# Frontend Interaction API

这份文档写给 web 前端开发人员。

目标不是解释 dora runtime 细节，而是明确一件事：

- `dm-input` 和 `dm-display` 是 `dm-server` 的客户端
- web 前端也是 `dm-server` 的客户端
- `dm-server` 是 interaction state 的唯一事实来源

也就是说，前端不要直接试图和 node 进程通信，也不要直接读 `~/.dm/runs/...` 目录。前端只通过 `dm-server` 暴露的 HTTP / WS 接口访问 interaction 数据。

## Mental Model

interaction 相关角色可以这样理解：

- `dm-input`
  - 向 server 注册“这个 run 里有哪些输入控件”
  - 从 server claim 用户输入事件
  - 把这些输入事件重新发回 dataflow
- `dm-display`
  - 把“这个 run 里有哪些可显示内容”通知给 server
  - 对于轻量内容，直接发送 inline content
  - 对于媒体/文件，发送 artifact 路径
- web 前端
  - 拉取当前 run 的 interaction state
  - 订阅 interaction 更新通知
  - 渲染 displays 和 inputs
  - 把用户操作通过 server 写回 input events
- `dm-server`
  - 保存 interaction state
  - 提供 artifact 文件访问
  - 提供 run / logs / status 等 API

## Base URL

默认 server 地址：

```text
http://127.0.0.1:3210
```

web 前端在浏览器里统一走：

```text
/api/...
```

例如：

```ts
await fetch("/api/runs/<run_id>/interaction")
```

## Core Endpoints

### 1. Get Interaction State

```http
GET /api/runs/{run_id}/interaction
```

返回当前 run 的 interaction snapshot。

示例响应：

```json
{
  "displays": [
    {
      "node_id": "reply",
      "label": "Assistant Reply",
      "kind": "inline",
      "content": "hello world",
      "render": "text",
      "tail": true,
      "max_lines": 500,
      "updated_at": 1775101235
    },
    {
      "node_id": "screen",
      "label": "Latest Screenshot",
      "kind": "file",
      "file": "artifacts/screenshots/latest.png",
      "render": "image",
      "tail": false,
      "max_lines": 500,
      "updated_at": 1775101236
    }
  ],
  "inputs": [
    {
      "node_id": "prompt",
      "label": "Prompt",
      "widgets": {
        "text": {
          "type": "textarea",
          "label": "User Prompt",
          "placeholder": "Type something..."
        }
      },
      "current_values": {
        "text": "hi"
      },
      "updated_at": 1775101159
    }
  ]
}
```

### 2. Emit User Input

```http
POST /api/runs/{run_id}/interaction/input/events
Content-Type: application/json
```

请求体：

```json
{
  "node_id": "prompt",
  "output_id": "text",
  "value": "hello"
}
```

用途：

- web 前端把用户操作写回 server
- `dm-input` 节点之后会从 server claim 到这条事件

### 3. Subscribe Interaction Notifications

```http
GET /api/runs/{run_id}/interaction/ws
```

用途：

- web 前端订阅 interaction 更新通知
- 收到通知后，再通过 HTTP 拉取最新 snapshot 或 message history

示例消息：

```json
{
  "event": "display.updated",
  "run_id": "092a3eed-3527-4c1e-a86e-a6a3cb6ce699",
  "source_id": "reply",
  "seq": null
}
```

注意：

- 这条 WS 是 notify-only
- 前端不要把 WS 当作完整数据源
- 真实数据仍然通过 HTTP 获取

### 4. Read Artifact File

```http
GET /api/runs/{run_id}/artifacts/{relative_path}
```

只用于 `kind = "file"` 的 display。

例子：

```text
/api/runs/<run_id>/artifacts/artifacts/screenshots/latest.png
```

注意：

- `relative_path` 必须是相对 `runs/{id}/out/` 的路径
- 前端不要拼绝对文件系统路径
- 前端不要假设本地磁盘可访问

### 5. Get Run Detail

```http
GET /api/runs/{run_id}
```

常用于 run 详情页初始化。

如果需要 metrics：

```http
GET /api/runs/{run_id}?include_metrics=true
```

### 6. Stop Run

```http
POST /api/runs/{run_id}/stop
```

用途：

- 停止当前 run
- 前端通常在 stop 成功后轮询 run detail，直到 `status !== "running"`

## Display Rendering Contract

`displays[]` 里最重要的是 `kind` 和 `render`。

### kind = "inline"

适用于：

- text
- json
- markdown
- 其他轻量可直接显示的数据

字段：

- `content`: 直接渲染内容
- `file`: 一般为空

前端处理：

- `render = "text"`: 直接显示字符串
- `render = "json"`: pretty-print JSON
- `render = "markdown"`: markdown 渲染

这类 display 不需要再请求 artifact。

### kind = "file"

适用于：

- image
- audio
- video
- 其他大文件或媒体文件

字段：

- `file`: 相对 `runs/{id}/out/` 的路径

前端处理：

- 通过 `/api/runs/{run_id}/artifacts/{file}` 获取资源
- `render = "image"`: `<img>`
- `render = "audio"`: `<audio>`
- `render = "video"`: `<video>`
- `render = "text" | "json" | "markdown"`: fetch 文本后渲染

## Input Rendering Contract

`inputs[]` 来自 `dm-input` 节点注册的 widget schema。

一个 input binding 结构如下：

```json
{
  "node_id": "prompt",
  "label": "Prompt",
  "widgets": {
    "text": {
      "type": "textarea",
      "label": "User Prompt"
    }
  },
  "current_values": {
    "text": "hi"
  }
}
```

理解方式：

- `node_id`: 这个 input node 的身份
- `widgets`: key 是 output port id
- 用户提交时要带：
  - `node_id`
  - `output_id`
  - `value`

也就是说，前端不需要理解 dataflow，只需要把控件值写回：

```json
{
  "node_id": "<binding.node_id>",
  "output_id": "<widget key>",
  "value": "<user value>"
}
```

## Recommended Frontend Flow

run 详情页推荐按下面的顺序工作：

1. 拉取 run 详情

```http
GET /api/runs/{run_id}
```

2. 拉取 interaction state

```http
GET /api/runs/{run_id}/interaction
```

3. 渲染 inputs / displays

4. 用户操作 input 时：

```http
POST /api/runs/{run_id}/interaction/input/events
```

5. 提交后重新拉 interaction state

```http
GET /api/runs/{run_id}/interaction
```

当前前端就是这个模型，本质上是 polling / refresh model，不是 websocket push model。

## Current Reality

当前 interaction 页是“定时刷新 run + interaction state”模式，不是实时订阅模式。

也就是说：

- 前端会周期性请求 `/api/runs/{id}`
- 前端会周期性请求 `/api/runs/{id}/interaction`
- `inline` displays 不需要额外 artifact fetch
- `file` displays 仍然需要 artifact fetch

如果未来要做更实时的体验，建议再加：

- interaction websocket / sse
- run 状态变更推送
- input ack / optimistic update

但这些都不是当前前端开发的前置条件。

## Rules For Frontend Developers

前端开发时请遵守这几个边界：

- 不直接读取 run 目录文件
- 不直接依赖 node 进程状态
- 不直接和 `dm-input` / `dm-display` 通信
- 一律通过 `dm-server` 的 `/api/...` 接口读写 interaction 状态
- `file` display 只使用相对路径，不拼本地绝对路径
- `inline` display 不再请求 artifact

## Minimal TypeScript Types

前端可以直接参考这组类型：

```ts
type DisplayEntry = {
  node_id: string;
  label: string;
  kind: "file" | "inline";
  file?: string | null;
  content?: any;
  render: "auto" | "text" | "json" | "markdown" | "image" | "audio" | "video";
  tail: boolean;
  max_lines: number;
  updated_at: number;
};

type InputBinding = {
  node_id: string;
  label: string;
  widgets: Record<string, any>;
  current_values: Record<string, any>;
  updated_at: number;
};

type InteractionSnapshot = {
  displays: DisplayEntry[];
  inputs: InputBinding[];
};
```

## Example Frontend Helpers

```ts
async function getInteraction(runId: string) {
  const res = await fetch(`/api/runs/${runId}/interaction`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function emitInteraction(runId: string, nodeId: string, outputId: string, value: any) {
  const res = await fetch(`/api/runs/${runId}/interaction/input/events`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      node_id: nodeId,
      output_id: outputId,
      value,
    }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

function artifactUrl(runId: string, file: string) {
  return `/api/runs/${runId}/artifacts/${file}`;
}
```

## Summary

前端开发人员只需要记住一句话：

web 不是在“操作节点”，而是在“消费和修改 server 里的 interaction state”。

节点和 web 都只是 `dm-server` 的客户端。
