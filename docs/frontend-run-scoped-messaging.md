# Frontend Run-Scoped Messaging Design

这份文档写给 web 前端开发人员。

核心结论：

- `dm-server` 提供的是 **run-scoped interaction messaging service**
- `dm-input`、`dm-display`、web 前端都只是这个服务的客户端
- 前端不直接面向 node 进程编程，而是面向 server 的 message store / query API 编程

## 1. Server Perspective

从 `dm-server` 视角，interaction 只有两类业务：

1. 消息存储
2. 消息分发

当前已经落地的是第一部分的基础形态：

- run 目录下持久化 interaction sqlite 数据库
- display source 最新状态持久化
- display message history 持久化
- input binding / input event 持久化

第二部分已经有了第一版实时通知：

- `server -> web` 通过 WS 发轻量通知
- web 收到通知后再通过 HTTP 拉 snapshot / history

这里的原则是：

- WS 负责通知
- HTTP 负责取数
- sqlite 是事实来源

## 2. Storage Layout

每个 run 在自己的目录下持久化 interaction 数据库：

```text
runs/<run_id>/interaction.db
```

当前 server 在这份 db 里维护至少这些数据：

- `display_messages`
  - append-only display 历史消息
- `display_sources`
  - 每个 display source 的最新状态
- `input_bindings`
  - 每个 input node 注册的 widgets 和 current_values
- `input_events`
  - web 发给 input node 的输入事件队列

这意味着前端可以依赖：

- snapshot 可恢复
- display 历史可查询
- 页面刷新后状态不会丢

## 3. Client Roles

### `dm-display`

职责：

- 把 dataflow 中要展示的内容发送给 server
- server 先存库，再更新 source 最新状态

两种输入模式：

- `path`
  - 文件类消息
  - 例如 screenshot / audio / video / artifact
- `data`
  - inline 消息
  - 例如 text / markdown / json

### `dm-input`

职责：

- 向 server 注册 input binding
- 从 server 读取用户输入事件
- 把用户输入重新发回 dataflow

### web 前端

职责：

- 拉取 snapshot
- 拉取 display message history
- 渲染 source
- 提交 input event
- 自己决定 display 的布局、切换、合并方式

注意：

- “把两个 display 合并到一个窗口”属于前端 composition 能力
- 这不是 server 的 canonical view model

## 4. Frontend Mental Model

前端应该把 server 暴露的数据理解成两层：

### A. Source Snapshot

回答的是：

- 当前 run 里有哪些 display source
- 当前 run 里有哪些 input source
- 每个 source 的最新状态是什么

对应接口：

```http
GET /api/runs/{run_id}/interaction
```

### B. Message History

回答的是：

- 某个 display source 说过什么
- 某个 run 里发生了哪些 display 消息
- 从某个 seq 之后新增了哪些消息

对应接口：

```http
GET /api/runs/{run_id}/interaction/messages
```

前端不要把 display 只理解成“当前值”，也要意识到它有 message-like history。

### C. Realtime Notification

回答的是：

- 当前 run 的 interaction 数据发生了变化
- 哪个 source 触发了变化

对应接口：

```http
GET /api/runs/{run_id}/interaction/ws
```

WS 只发轻量通知，不承载完整 display 内容。前端收到通知后，应自行调用：

- `GET /api/runs/{run_id}/interaction`
- 或 `GET /api/runs/{run_id}/interaction/messages?...`

## 5. Implemented APIs

### 5.1 Get Interaction Snapshot

```http
GET /api/runs/{run_id}/interaction
```

返回：

```ts
type InteractionSnapshot = {
  displays: DisplayEntry[];
  inputs: InputBinding[];
};
```

### 5.2 Get Display Messages

```http
GET /api/runs/{run_id}/interaction/messages?after_seq=0&source_id=reply&limit=200
```

查询参数：

- `after_seq` 可选
- `source_id` 可选
- `limit` 可选

返回：

```ts
type DisplayMessage = {
  seq: number;
  node_id: string;
  label: string;
  kind: "file" | "inline";
  file?: string | null;
  content?: any;
  render: "text" | "json" | "markdown" | "image" | "audio" | "video";
  tail: boolean;
  max_lines: number;
  created_at: number;
};

type DisplayMessagesResponse = {
  messages: DisplayMessage[];
  next_seq: number;
};
```

### 5.3 Emit Input Event

```http
POST /api/runs/{run_id}/interaction/input/events
```

请求体：

```json
{
  "node_id": "prompt",
  "output_id": "text",
  "value": "hello"
}
```

### 5.4 Interaction Notify-Only WebSocket

```http
GET /api/runs/{run_id}/interaction/ws
```

示例通知：

```json
{
  "event": "display.updated",
  "run_id": "092a3eed-3527-4c1e-a86e-a6a3cb6ce699",
  "source_id": "reply",
  "seq": null
}
```

当前事件类型包括：

- `display.updated`
- `input.binding.updated`
- `input.event.created`

### 5.5 Read Artifact File

```http
GET /api/runs/{run_id}/artifacts/{relative_path}
```

只适用于：

- `kind = "file"`

不适用于：

- `kind = "inline"`

## 6. Data Contracts

### Display Source

```ts
type DisplayEntry = {
  node_id: string;
  label: string;
  kind: "file" | "inline";
  file?: string | null;
  content?: any;
  render: "text" | "json" | "markdown" | "image" | "audio" | "video";
  tail: boolean;
  max_lines: number;
  updated_at: number;
};
```

规则：

- `kind = "inline"` 时，前端直接渲染 `content`
- `kind = "file"` 时，前端通过 artifact API 读取 `file`

### Input Binding

```ts
type InputBinding = {
  node_id: string;
  label: string;
  widgets: Record<string, any>;
  current_values: Record<string, any>;
  updated_at: number;
};
```

规则：

- `widgets` 的 key 就是要回写给 server 的 `output_id`
- 前端提交输入时只需要带：
  - `node_id`
  - `output_id`
  - `value`

## 7. Frontend Composition Responsibility

前端负责：

- 一个窗口显示哪个 source
- 是否允许 source switch
- 是否把多个 source 合并成 tabs / split / timeline / conversation
- 这些布局和偏好如何保存

server 不负责：

- 规定窗口布局
- 规定 source 合并方式
- 规定“哪个 display 必须和哪个 display 合并”

所以前端拿到的是 source model，不是 view layout model。

## 8. Recommended Frontend Flow

### 初次进入 run 页面

1. 请求 run detail
2. 请求 interaction snapshot
3. 如果页面需要历史消息，再请求 display messages

### 用户输入

1. 前端发送 input event
2. 前端重新拉 interaction snapshot，或等待后续 push

### 文件类 display

1. 从 snapshot 拿到 `file`
2. 用 artifact API 读取

### 文本类 display

1. 从 snapshot 直接拿 `content`
2. 直接渲染

## 9. Current State vs Next State

### Current

当前已实现：

- sqlite-backed run interaction store
- snapshot API
- display history API
- input event write API
- artifact read API

### Next

下一阶段建议补：

- `server -> web` 的 WS/SSE 推送
- `server -> dm-input` 的 WS 下发通道

但前端不应该围绕“轮询”建模。
前端应该围绕：

- snapshot
- messages
- push notification

这三个概念建模。

## 10. Practical Rule

前端开发时只记住一句话：

**web 不是在和节点通信，而是在消费 run-scoped messaging service。**

节点和 web 都只是 `dm-server` 的客户端。
