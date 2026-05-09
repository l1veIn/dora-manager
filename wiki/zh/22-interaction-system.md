交互系统是 Dora Manager 中连接人类与数据流的桥梁层。它的核心架构决策是让交互节点通过 **dm Python SDK**（`dm.Message()`）直接与 dm-server 通信，取代了旧的隐藏 Bridge 节点模式。交互节点只需通过 HTTP API 发送展示消息和接收用户输入，无需任何网络栈代码——SDK 封装了所有 HTTP 通信细节。本文将从**SDK 双端口模型**出发，深入解析展示型节点如何使用 `msg.send()`、输入型节点如何使用后台线程轮询 `msg.get()`、以及 SDK 如何自动发现运行时上下文。

Sources: [\_message.py](https://github.com/l1veIn/dora-manager/blob/main/sdk/python/dm/_message.py#L1-L167), [dm-display/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm_display/main.py#L1-L172), [dm-slider/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm_slider/main.py#L1-L90)

## SDK 双端口模型：展示与输入

交互系统的架构基础是一个明确的**双端口模型**。每个使用 SDK 的交互节点拥有两种通信模式：

- **展示（outbound）端口**：通过 `msg.send(tag, payload)` 将内容发送到 dm-server，前端通过 `message_snapshots` API 获取最新值并渲染
- **输入（inbound）端口**：通过后台线程轮询 `msg.get(tag="input", after_seq=LAST_SEQ)` 获取用户输入，过滤出目标为自己的消息后通过 dora 标准输出端口发送

这两种端口在节点的 `dm.json` 中通过 `capabilities` 字段声明。`display` 族声明展示能力，`widget_input` 族声明输入能力。SDK 节点无需在 dora YAML 中声明额外的输入或输出端口——所有交互通信独立于 dora 数据平面。

```
graph TB
    subgraph "Data Plane (dora)"
        Compute["计算节点"]
        Display["dm-display"]
        Input["dm-slider / dm-button / dm-text-input / dm-input-switch"]
    end

    subgraph "DM Plane (HTTP)"
        Server["dm-server"]
        Web["Web Browser"]
    end

    Compute -- "Arrow (path/data)" --> Display
    Display -- "HTTP POST /api/runs/.../messages" --> Server
    Input -- "HTTP GET /api/runs/.../messages (轮询)" --> Server
    Server -- "WebSocket notify" --> Web
    Web -- "HTTP POST" --> Server
    Input -- "Arrow (value/click)" --> Compute
```

**交互节点不直接暴露网络端口**。dm-display 通过 SDK 的 `msg.send()` 调用 HTTP API，dm-slider 通过 SDK 的 `msg.get()` 轮询用户输入。所有与 dm-server 的 HTTP 通信细节都由 SDK 封装在 `dm.Message` 类中。

Sources: [dm-display/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm.json#L41-L75), [dm-slider/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm.json#L37-L56)

## dm-display：展示型节点的工作原理

**dm-display** 是交互族的展示侧节点，负责将 data-plane 中需要人类查看的内容转发到 DM-plane。它有两个标准 dora 输入端口——`path`（文件路径）和 `data`（内联内容）——然后通过 SDK 的 `msg.send()` 将内容推送到 dm-server。

### 双端口输入与 SDK 展示

dm-display 的工作流极为简洁：接收 dora INPUT 事件，构造一个 `{tag, payload}` 格式的 JSON 消息，通过 SDK 发送出去。具体而言，当 `path` 端口收到文件路径时，payload 包含 `{kind: "file", file: "<相对路径>"}`；当 `data` 端口收到内联内容时，payload 包含 `{kind: "inline", content: "<内容>"}`。`tag` 字段始终使用渲染模式名称（如 `"text"`、`"image"`、`"json"`）。

核心发送逻辑通过 SDK 完成：

```python
msg = dm.Message()
msg.send(tag, {"kind": kind, "content": content}, from_=node_id)
```

SDK 的 `send` 方法自动从环境变量读取 `DM_RUN_ID` 和 `DM_NODE_ID`，构造正确的 HTTP POST 请求到 `/api/runs/{run_id}/messages`，无需节点手动处理 URL 或认证。

Sources: [dm-display/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm_display/main.py#L1-L172)

### 渲染模式自动推断

dm-display 的 `render` 配置项支持 `"auto"` 模式，此时根据输入来源自动选择渲染方式。对于 `path` 输入，通过文件扩展名映射（`.log` → `text`、`.json` → `json`、`.png` → `image` 等）；对于 `data` 输入，根据 Python 值类型推断（`dict`/`list` → `json`，其他 → `text`）。开发者也可通过 `RENDER` 环境变量强制指定。

Sources: [dm-display/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm_display/main.py#L15-L29)

## dm-input 家族：输入型节点的工作原理

输入型节点是交互系统的"人→数据流"方向桥梁。当前内置四种，它们遵循完全相同的工作模式：**在后台线程中通过 SDK 轮询用户输入消息，解析后通过标准 dora 输出端口发送 Arrow 数据**。

### 输入节点对照表

| 节点 | dm.json 声明的 Capability | 输出端口 | 输出 Arrow 类型 | 典型用途 |
|------|--------------------------|---------|----------------|---------|
| **dm-text-input** | `widget_input` | `value` | `utf8` | 文本提示、多行输入 |
| **dm-button** | `widget_input` | `click` | `utf8` | 触发动作、流程控制 |
| **dm-slider** | `widget_input` | `value` | `float64` | 数值调节、参数控制 |
| **dm-input-switch** | `widget_input` | `value` | `boolean` | 开关切换、模式选择 |

Sources: [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-text-input/dm.json#L37-L55), [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-button/dm.json#L37-L55), [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm.json#L37-L56), [dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-input-switch/dm.json#L37-L56)

### 统一的消息轮询模式

所有输入节点的核心逻辑完全一致——在后台线程中通过 SDK 轮询用户输入消息，过滤出目标为自己的消息，类型转换后发送到语义输出端口。以 dm-slider 为例：

```python
def poll_inputs(msg, node, yaml_id):
    last_seq = 0
    while node.is_running():
        messages = msg.get(tag="input", after_seq=last_seq)
        for item in messages:
            last_seq = item["seq"]
            payload = item["payload"]
            if payload.get("to") == yaml_id:
                node.send_output("value", pa.array([float(payload["value"])]))
        time.sleep(0.1)

msg = dm.Message()
thread = threading.Thread(target=poll_inputs, args=(msg, node, yaml_id), daemon=True)
thread.start()
```

每个节点的差异仅在 `normalize_output`（类型转换）函数中——dm-slider 将值转换为 `float64`，dm-input-switch 转换为 `boolean`，dm-text-input 和 dm-button 保持为 `utf8` 字符串。SDK 的 `msg.get()` 方法自动从 `after_seq` 参数指定的序列号之后的消息开始轮询，确保每条消息只处理一次。

Sources: [dm-slider/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm_slider/main.py#L1-L90), [dm-button/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-button/dm_button/main.py#L1-L81), [dm-input-switch/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-input-switch/dm_input_switch/main.py#L1-L77), [dm-text-input/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-text-input/dm_text_input/main.py#L1-L104)

## SDK 自动发现运行时上下文

SDK 的 `dm.Message` 类在初始化时自动从环境变量发现运行时上下文：

| 环境变量 | 用途 |
|----------|------|
| `DM_RUN_ID` | 当前运行实例的唯一标识符，用于构造 HTTP API 路径 |
| `DM_NODE_ID` | 节点在数据流中的身份标识，用于 `msg.send()` 的 `from_` 参数和 `msg.get()` 的输入过滤 |

这意味着节点代码无需手动传递或硬编码这些值——SDK 在第一次调用时会自动读取 `os.environ`。如果环境中缺少这些变量，`msg.send()` 和 `msg.get()` 会抛出清晰的错误提示。

Sources: [\_message.py](https://github.com/l1veIn/dora-manager/blob/main/sdk/python/dm/_message.py#L1-L167)

## Widget 注册：通过 msg.send("widgets", ...)

交互节点在启动时通过 SDK 注册控件（widget）描述，使前端能够自动渲染对应的 UI 控件。注册发生在节点主循环开始之前：

```python
def main():
    msg = dm.Message()
    node = Node()

    # 注册控件——节点启动时立即发送
    msg.send("widgets", {
        "label": "Temperature (°C)",
        "widgets": {
            "value": {
                "type": "slider",
                "label": "Temperature",
                "min": -20, "max": 50, "step": 1, "default": 20
            }
        }
    })

    # 启动后台线程轮询输入
    thread = threading.Thread(target=poll_inputs, args=(msg, node, yaml_id), daemon=True)
    thread.start()

    # 主循环——处理 dora Arrow 输入（可选）
    for event in node:
        ...
```

SDK 节点的 widget 注册通过 `msg.send("widgets", payload)` 完成，与普通消息共享同一 HTTP API 端点。dm-server 将其作为 `tag="widgets"` 的快照存储，前端通过 `GET /api/runs/{id}/messages/snapshots` 获取并渲染。

Sources: [dm_sdk_demo/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-sdk-demo/dm_sdk_demo/main.py#L1-L95)

## dm-server 的消息服务

dm-server 的 `MessageService` 基于 SQLite 实现消息持久化。每条 `push` 操作同时写入 `messages` 历史表（append-only）和更新 `message_snapshots` 快照表（通过 `UPSERT` 语义确保每个 `(node_id, tag)` 组合总是保存最新状态）。这种双表设计同时支持历史回溯和快速快照查询。

```
sequenceDiagram
    participant Node as SDK 节点
    participant API as dm-server REST
    participant DB as SQLite
    participant WS as WebSocket
    participant Web as Web Browser

    Note over Node,Web: 初始化阶段
    Node->>API: POST /api/runs/{id}/messages {"tag":"widgets","payload":{...}}
    API->>DB: MessageService.push() → upsert snapshots
    API->>WS: broadcast notification

    Note over Node,Web: 展示链路
    Node->>API: POST /api/runs/{id}/messages {"tag":"text","payload":{...}}
    API->>DB: INSERT messages + UPSERT snapshots
    API->>WS: broadcast notification
    Web->>DB: GET /snapshots → 获取最新快照

    Note over Node,Web: 输入链路
    Web->>API: POST /api/runs/{id}/messages {"tag":"input","payload":{to:"slider",value:25}}
    API->>DB: INSERT messages + UPSERT snapshots
    API->>WS: broadcast notification
    Node->>API: GET /messages?tag=input&after_seq=N
    Node->>Node: 过滤 payload.to == yaml_id → send_output
```

Sources: [message.rs](https://github.com/l1veIn/dora-manager/blob/main/crates/dm-server/src/services/message.rs#L1-L243)

## Capability Binding 声明规范

交互节点的能力由 `dm.json` 中的 `capabilities` 字段声明，SDK 节点通过其元数据被系统识别。当前支持两个 capability 族：

### `display` 族

声明节点是展示型节点，桥接 data-plane 内容到 DM-plane。每个 binding 声明一个数据通道：

| 字段 | 含义 | 典型值 |
|------|------|--------|
| `role` | 节点在该族中的角色 | `"source"` |
| `port` | dora 端口名，标识 data-plane 与 DM-plane 的交汇点 | `"data"` 或 `"path"` |
| `channel` | DM 侧的语义通道 | `"inline"` 或 `"artifact"` |
| `media` | 支持的渲染类型列表 | `["text", "json", "markdown"]` |
| `lifecycle` | 生命周期提示 | `[]` |

Sources: [dm-display/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-display/dm.json#L41-L75), [dm-capability-binding-v0.md](https://github.com/l1veIn/dora-manager/blob/main/docs/design/dm-capability-binding-v0.md#L56-L100)

### `widget_input` 族

声明节点是输入型节点，接收 DM-plane 的用户操作并注入 data-plane。每个 binding 声明一个交互通道：

| 字段 | 含义 | 典型值 |
|------|------|--------|
| `role` | 节点角色 | `"widget"` |
| `channel` | 通道语义：`"register"` 为注册，`"input"` 为数据接收 | `"register"` / `"input"` |
| `port` | dora 输出端口名（仅 `channel=input` 时存在） | `"value"` / `"click"` |
| `media` | 交互数据类型 | `["text"]`、`["number"]`、`["boolean"]`、`["pulse"]` |
| `lifecycle` | 生命周期约束 | `["run_scoped", "stop_aware"]` |

每个 widget_input 节点通常声明两个 binding：一个 `channel: "register"` 描述 widget 注册信息，一个 `channel: "input"` 描述数据接收端口。SDK 节点根据这两个 binding 分别进行 widget 注册（启动时 `msg.send("widgets", ...)`）和输入轮询（后台线程 `msg.get(tag="input", ...)`）。

Sources: [dm-slider/dm.json](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-slider/dm.json#L37-L56), [dm-capability-binding-v0.md](https://github.com/l1veIn/dora-manager/blob/main/docs/design/dm-capability-binding-v0.md#L102-L148)

## 扩展自定义交互节点

创建新的交互节点遵循严格的契约，且无需修改 dm-core 代码：

**创建新的展示型节点**：
1. 在 `dm.json` 中声明 `display` capability，指定 `port` 和 `channel`
2. 节点实现中，初始化 `dm.Message()`，通过 `msg.send(tag, payload)` 推送内容
3. SDK 自动处理 HTTP 通信和运行 ID 发现

**创建新的输入型节点**：
1. 在 `dm.json` 中声明 `widget_input` capability，指定 `channel: "register"` 和 `channel: "input"` 的 binding
2. 节点实现中，启动后台线程通过 `msg.get(tag="input", after_seq=...)` 轮询输入
3. 过滤 `payload.to == yaml_id`，类型转换后通过声明的输出端口发送

**Widget 类型注册**：在节点代码中使用 `msg.send("widgets", payload)` 发送控件描述。SDK 节点的 widget 类型映射表由前端组件库定义，当前支持 input、textarea、button、select、slider、switch、radio、checkbox、path、file 等类型。

Sources: [dm-sdk-demo/main.py](https://github.com/l1veIn/dora-manager/blob/main/nodes/dm-sdk-demo/dm_sdk_demo/main.py#L1-L95), [InteractionPane.svelte](https://github.com/l1veIn/dora-manager/blob/main/web/src/routes/runs/[id]/InteractionPane.svelte#L200-L321)

## 相关阅读

- [Capability Binding：节点能力声明与运行时角色绑定](23-capability-binding-jie-dian-neng-li-sheng-ming-yu-yun-xing-shi-jiao-se-bang-ding) — 深入了解 capability 体系的完整设计
- [内置节点总览：从媒体采集到 AI 推理](7-nei-zhi-jie-dian-zong-lan-cong-mei-ti-cai-ji-dao-ai-tui-li) — 了解交互节点在完整节点生态中的定位
- [响应式控件（Widgets）：控件注册表、动态渲染与 WebSocket 参数注入](20-xiang-ying-shi-kong-jian-widgets-kong-jian-zhu-ce-biao-dong-tai-xuan-ran-yu-websocket-can-shu-zhu-ru) — 前端 widget 渲染机制的详细文档
