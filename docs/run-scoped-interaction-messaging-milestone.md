# Run-Scoped Interaction Messaging Milestone

> Status: implemented milestone

这份文档写给后续维护 `dm-server`、interaction nodes、web 前端的开发者。

它总结这次重构后的设计视角、边界、理念，以及当前已经落地的实现。

## 1. 结论

我们现在不再从 `panel` 视角思考交互，而是从 `dm-server` 视角思考：

- `dm-server` 提供一个 **run-scoped interaction messaging service**
- `dm-display`、`dm-input`、web 前端都是这个服务的客户端
- 核心业务只有两个：
  - 消息存储
  - 消息推送

这不是 `dm-core` 的职责。

## 2. 分层边界

### `dm-core`

负责：

- run lifecycle
- run repo layout
- dataflow transpile
- node runtime orchestration
- 运行时环境变量注入，例如 `DM_RUN_ID`、`DM_RUN_OUT_DIR`、`DM_SERVER_URL`、`DM_NODE_ID`

不负责：

- interaction message schema
- interaction database
- websocket protocol
- web-facing query / subscription API

### `dm-server`

负责：

- run-scoped interaction database
- interaction HTTP API
- interaction websocket delivery
- message history / snapshot query
- input / display client 接入

## 3. 设计原则

### 3.1 server-first

我们不再从“页面上要有一个 panel”反推架构，而是先定义 server 能力：

- store
- query
- notify
- deliver

UI 只是其中一个 client。

### 3.2 DB is source of truth

真实状态先落库，再暴露给客户端。

- web 不依赖内存态
- websocket 不承载唯一事实来源
- 断线恢复依赖数据库 + HTTP 查询

### 3.3 notify vs fetch 分离

`server -> web` 采用：

- WS 负责通知
- HTTP 负责取数

web 收到通知后，再调用 HTTP 拉最新 snapshot 或 message history。

### 3.4 上下行链路分开建模

两条链路不是一回事：

- display 链路：`dataflow -> dm-display -> dm-server -> web`
- input 链路：`web -> dm-server -> dm-input -> dataflow`

所以协议也分开：

- `server -> web`：notify-only WS
- `server -> dm-input`：data-carrying WS

### 3.5 storage 与 display 解耦

并不是所有 display 都必须先写文件。

- `image/audio/video/large artifact` 适合 file-based display
- `text/json/markdown/small structured data` 适合 inline display

所以现在 `dm-display` 同时支持：

- `kind = "file"`
- `kind = "inline"`

## 4. 当前实现

### 4.1 run-scoped database

每个 run 维护：

```text
runs/<run_id>/interaction.db
```

当前表结构：

- `display_messages`
  - append-only display history
- `display_sources`
  - latest display state per source
- `input_bindings`
  - registered widgets and current values
- `input_events`
  - append-only input events

### 4.2 已实现的 HTTP API

#### Snapshot / History

- `GET /api/runs/{id}/interaction`
- `GET /api/runs/{id}/interaction/messages`

#### Display writes

- `POST /api/runs/{id}/interaction/display`

#### Input writes / reads

- `POST /api/runs/{id}/interaction/input/register`
- `POST /api/runs/{id}/interaction/input/events`
- `GET /api/runs/{id}/interaction/input/claim/{node_id}`

#### Artifact reads

- `GET /api/runs/{id}/artifacts/{relative_path}`

### 4.3 已实现的 websocket

#### A. web notify-only websocket

```text
GET /api/runs/{id}/interaction/ws
```

用途：

- 通知 web：interaction 数据发生变化

当前事件：

- `display.updated`
- `input.binding.updated`
- `input.event.created`

消息示例：

```json
{
  "event": "display.updated",
  "run_id": "092a3eed-3527-4c1e-a86e-a6a3cb6ce699",
  "source_id": "reply",
  "seq": 12
}
```

注意：

- 这条 WS 不承载完整 display 内容
- web 收到后要自己再调 HTTP

#### B. input node downstream websocket

```text
GET /api/runs/{id}/interaction/input/ws/{node_id}?since=<seq>
```

用途：

- `dm-input` 订阅属于自己的输入事件
- 连接建立时会先 replay `since` 之后的历史事件
- 连接建立后继续接收新的 live event

消息示例：

```json
{
  "type": "input.event",
  "event": {
    "seq": 3,
    "node_id": "prompt",
    "output_id": "text",
    "value": "hello",
    "timestamp": 1775101235
  }
}
```

## 5. 节点角色

### `dm-display`

角色：

- interaction messaging service 的 display producer

行为：

- `file` 模式下把 artifact 相对路径发给 server
- `inline` 模式下把轻量内容直接发给 server

当前保留的轮询：

- 只有 `source` 静态文件模式还会轮询文件变化
- 这是节点内部实现细节，不是前端契约

### `dm-input`

角色：

- interaction messaging service 的 input consumer

行为：

- 启动时 HTTP 注册 widgets
- 之后通过 input WS 接收 input events
- 不再通过 HTTP claim 轮询消费事件
- 收到事件后把值重新发回 dataflow

## 6. Web 前端心智模型

前端应该把自己看成 `dm-server` 的客户端，而不是某个 node 的控制面板。

推荐模型：

1. 页面初始化
2. `GET /api/runs/{id}/interaction`
3. 建立 `GET /api/runs/{id}/interaction/ws`
4. 收到通知后重新拉 HTTP 数据

前端负责：

- 渲染 display sources
- 渲染 input bindings
- display source 的切换、合并、布局、窗口组织

server 不负责：

- canonical panel layout
- display composition policy
- UI workspace state

## 7. 为什么这不是“重新发明 panel”

相似点是：

- 都涉及 run 级持久化
- 都涉及交互消息
- 都涉及 web 展示

本质差异是：

- 旧 panel 是从 UI 出发反推系统
- 现在是从 server capability 出发定义客户端协议
- 旧 panel 容易把 UI 语义侵入 core
- 现在明确限定在 `dm-server`
- 旧 panel 倾向于 server 持有具体 view 模型
- 现在 server 只持有 source state 和 message log，前端自己做 composition

## 8. 当前状态

已经完成：

- panel 耦合移除
- storage family / interaction family 第一版重建
- inline display + file display
- sqlite-backed interaction store
- web notify-only websocket
- input downstream websocket

当前仍保留但不再是主路径：

- `GET /interaction/input/claim/{node_id}`
  - 兼容旧实现
  - 现在不是首选消费路径

## 9. 后续建议

下一阶段建议继续做：

1. 为 interaction websocket 增加更清晰的 event schema versioning
2. 为 web 增加 `after_seq` 增量拉取策略，减少整包 snapshot 刷新
3. 视需要为 `dm-display` 的静态 `source` 模式去掉文件轮询，改成更显式的更新源
4. 把前端的 display composition / workspace persistence 单独设计，不回流到 server

## 10. 参考文件

- [crates/dm-server/src/interaction_service.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-server/src/interaction_service.rs)
- [crates/dm-server/src/handlers/interaction.rs](/Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/interaction.rs)
- [nodes/dm-display/dm_display/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-display/dm_display/main.py)
- [nodes/dm-input/dm_input/main.py](/Users/yangchen/Desktop/dora-manager/nodes/dm-input/dm_input/main.py)
- [docs/frontend-run-scoped-messaging.md](/Users/yangchen/Desktop/dora-manager/docs/frontend-run-scoped-messaging.md)
