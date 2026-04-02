# Interaction Family: dm-display & dm-input

> Status: **Design document**

## 架构定位

交互族节点是人类与数据流之间的桥梁。它们不做计算、不做存储，只做一件事：**桥接人机**。

```
┌─────────────────────────────────────────────────────────────┐
│ dora dataflow                                               │
│                                                             │
│  compute ──→ dm-log ──→ filesystem ──→ dm-display           │
│                                            │ (HTTP POST)    │
│  compute ←── dm-input ←────────────────────┼────────────    │
│                                            │                │
└────────────────────────────────────────────┼────────────────┘
                                             │
                                      ┌──────┴──────┐
                                      │  dm-server   │
                                      │  (IPC 中继)  │
                                      │  唯一服务器   │
                                      └──────┬──────┘
                                             │ WebSocket
                                      ┌──────┴──────┐
                                      │  Web 浏览器   │
                                      │  展示 + 控件  │
                                      └─────────────┘
```

### 核心原则

1. **展示侧不碰 Arrow** — dm-display 读取存储族节点已持久化的产物，不序列化 Arrow
2. **节点不起服务器** — 节点是 dm-server 的轻量客户端，不监听端口
3. **上下行解耦** — 展示和输入是两个独立节点，可单独使用也可组合
4. **dm-server 是唯一中继** — 所有人机通信经过 dm-server

---

## Node 1: dm-display

### 职责

读取存储族节点的持久化产物 → 通知 dm-server "有新内容" → server 推送给 Web 渲染。

dm-display **不做序列化**。它接收的输入是文件路径（dm-log/dm-save/dm-recorder 的 `path` 输出），而非原始 Arrow 数据。

### 数据流

```
dm-log ──(path output)──→ dm-display ──(HTTP POST)──→ dm-server ──(WS push)──→ 浏览器
dm-save ──(path output)──→ dm-display
```

### 接口

```yaml
- id: show-chat
  node: dm-display
  inputs:
    path: log-chat/path          # 来自 dm-log 的文件路径输出
  config:
    label: "Chat Log"            # 展示面板标题
    render: "auto"               # auto | text | image | audio | video | json | markdown
    tail: true                   # true = 实时追尾显示; false = 全量刷新
    max_lines: 500               # text/json 模式下最大显示行数
```

也支持直接指定静态路径（不接受输入，定时重新读取）：

```yaml
- id: show-result
  node: dm-display
  config:
    label: "Latest Result"
    source: "result.json"        # 相对于 runs/:id/out/ 的静态路径
    render: json
    poll_interval: 2000          # 毫秒，定时重读
```

### Config

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | string | node ID | 展示面板的显示标题 |
| `render` | string | `"auto"` | 渲染模式。`auto` 根据文件扩展名推断 |
| `tail` | bool | `true` | 文本文件是否只显示尾部（追尾） |
| `max_lines` | int | `500` | 文本模式下最大显示行数 |
| `source` | string/null | `null` | 静态源文件路径（无 input 时使用） |
| `poll_interval` | int | `2000` | 静态源轮询间隔 (ms) |

### 渲染模式映射

| render | 文件类型 | Web 端渲染 |
|--------|---------|-----------|
| `text` | .log, .txt | 滚动文本面板 (monospace) |
| `json` | .json | 格式化 JSON viewer |
| `markdown` | .md | Markdown 渲染 |
| `image` | .png, .jpg | 图片展示（自动刷新最新帧） |
| `audio` | .wav, .mp3 | 音频播放器 |
| `video` | .mp4 | 视频播放器 |
| `auto` | any | 根据扩展名匹配上述规则，未知则 text |

### 节点实现

```python
from dora import Node
import os, sys, time, requests

node = Node()
cfg = node.config
run_id = os.environ["DM_RUN_ID"]
server_url = os.environ.get("DM_SERVER_URL", "http://127.0.0.1:31820")
run_out = os.environ.get("DM_RUN_OUT_DIR", ".")

label = cfg.get("label", node.node_id())
render_mode = cfg.get("render", "auto")
source = cfg.get("source")

def notify_server(file_path, render):
    """Lightweight HTTP POST to dm-server: 'new content available'."""
    try:
        requests.post(
            f"{server_url}/api/runs/{run_id}/display",
            json={
                "node_id": node.node_id(),
                "label": label,
                "file": file_path,               # relative to runs/:id/out/
                "render": render,
                "tail": cfg.get("tail", True),
                "max_lines": cfg.get("max_lines", 500),
                "timestamp": time.time(),
            },
            timeout=2,
        )
    except Exception as e:
        print(f"[dm-display] Server notify failed: {e}", file=sys.stderr)


def resolve_render(path):
    if render_mode != "auto":
        return render_mode
    ext = os.path.splitext(path)[1].lower()
    return {
        ".log": "text", ".txt": "text",
        ".json": "json", ".md": "markdown",
        ".png": "image", ".jpg": "image", ".jpeg": "image",
        ".wav": "audio", ".mp3": "audio",
        ".mp4": "video",
    }.get(ext, "text")


# Mode A: input-driven (receives path from storage node)
if source is None:
    for event in node:
        if event["type"] != "INPUT" or event["id"] != "path":
            continue
        file_path = event["value"].as_py()
        if isinstance(file_path, list):
            file_path = file_path[0]
        if isinstance(file_path, bytes):
            file_path = file_path.decode()

        # Make relative to run out dir
        rel = os.path.relpath(file_path, run_out) if os.path.isabs(file_path) else file_path
        notify_server(rel, resolve_render(rel))
        print(f"[DM-IO] DISPLAY {resolve_render(rel)} -> {rel}")

# Mode B: static source (poll file directly)
else:
    poll_ms = cfg.get("poll_interval", 2000)
    last_mtime = 0
    while True:
        full_path = os.path.join(run_out, source)
        if os.path.exists(full_path):
            mtime = os.path.getmtime(full_path)
            if mtime > last_mtime:
                last_mtime = mtime
                notify_server(source, resolve_render(source))
                print(f"[DM-IO] DISPLAY {resolve_render(source)} -> {source}")
        time.sleep(poll_ms / 1000)
```

### 依赖

```
requests
```

---

## Node 2: dm-input

### 职责

连接 dm-server → 监听用户在 Web 前端的控件操作 → 序列化为 Arrow → 输出给数据流中的后续节点。

dm-input 是一个**数据源节点**——它产出数据，数据来源是人类。

### 数据流

```
浏览器(控件操作) ──(WS)──→ dm-server ──(polling/WS)──→ dm-input ──(Arrow)──→ 计算节点
```

### 接口

```yaml
- id: user-prompt
  node: dm-input
  outputs: [text]
  config:
    label: "Prompt Input"
    widgets:
      text:
        type: textarea
        placeholder: "Enter your prompt..."
        hotkey: "Enter"

- id: controls
  node: dm-input
  outputs: [action, volume]
  config:
    label: "Control Panel"
    widgets:
      action:
        type: button
        label: "Start Processing"
        hotkey: "Ctrl+Enter"
      volume:
        type: slider
        min: 0
        max: 100
        default: 50
```

### Config

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | string | node ID | 控制面板标题 |
| `widgets` | object | **required** | Widget 声明，key = output port name |

### Widget 类型

| type | 输出类型 | 描述 |
|------|---------|------|
| `textarea` | string | 多行文本输入 |
| `input` | string | 单行文本输入 |
| `button` | string | 按钮，输出 label 文本 |
| `select` | string | 下拉选择 |
| `slider` | number | 滑块 |
| `switch` | boolean | 开关 |
| `radio` | string | 单选 |
| `checkbox` | string[] | 多选 |
| `file` | bytes | 文件上传 |

每个 widget 的通用可选字段：

| Field | Type | Description |
|-------|------|-------------|
| `label` | string | 控件显示标签 |
| `placeholder` | string | 占位提示文本 |
| `default` | any | 默认值 |
| `hotkey` | string | 快捷键绑定 |
| `options` | string[] | select/radio/checkbox 的选项 |
| `min`, `max`, `step` | number | slider 的范围参数 |

### 节点实现

```python
from dora import Node
import os, sys, time, json, requests
import pyarrow as pa

node = Node()
cfg = node.config
run_id = os.environ["DM_RUN_ID"]
server_url = os.environ.get("DM_SERVER_URL", "http://127.0.0.1:31820")

label = cfg.get("label", node.node_id())
widgets = cfg.get("widgets", {})

# Step 1: Register widgets with dm-server on startup
try:
    requests.post(
        f"{server_url}/api/runs/{run_id}/input/register",
        json={
            "node_id": node.node_id(),
            "label": label,
            "widgets": widgets,
        },
        timeout=5,
    )
except Exception as e:
    print(f"[dm-input] Failed to register widgets: {e}", file=sys.stderr)
    sys.exit(1)

# Step 2: Poll dm-server for user commands
poll_url = f"{server_url}/api/runs/{run_id}/input/{node.node_id()}/poll"
last_seq = 0

for event in node:
    # On every dora tick (or use a timer input to drive polling)
    if event["type"] == "INPUT" and event["id"] == "tick":
        try:
            resp = requests.get(
                poll_url,
                params={"since": last_seq},
                timeout=2,
            )
            if resp.status_code == 200:
                commands = resp.json().get("commands", [])
                for cmd in commands:
                    output_id = cmd["output_id"]    # matches widget key / output port
                    value = cmd["value"]
                    last_seq = max(last_seq, cmd.get("seq", last_seq))

                    # Serialize and send as Arrow output
                    if isinstance(value, str):
                        node.send_output(output_id, pa.array([value]))
                    elif isinstance(value, (int, float)):
                        node.send_output(output_id, pa.array([value]))
                    elif isinstance(value, bool):
                        node.send_output(output_id, pa.array([value]))
                    elif isinstance(value, list):
                        node.send_output(output_id, pa.array(value))

                    print(f"[DM-IO] INPUT {output_id} = {value}")
        except Exception as e:
            print(f"[dm-input] Poll failed: {e}", file=sys.stderr)
```

### 依赖

```
requests
```

---

## dm-server 新增端点

### Display 通知

```
POST /api/runs/:id/display
```

```json
{
  "node_id": "show-chat",
  "label": "Chat Log",
  "file": "chat.log",
  "render": "text",
  "tail": true,
  "max_lines": 500,
  "timestamp": 1711972800.0
}
```

Server 收到后：
1. 存入内存中该 run 的 display 注册表
2. 通过已有的 run WebSocket 推送给所有连接的 Web 客户端
3. Web 收到后，从 `GET /api/runs/:id/out/{file}` 拉取文件内容渲染

### Input 注册

```
POST /api/runs/:id/input/register
```

```json
{
  "node_id": "user-prompt",
  "label": "Prompt Input",
  "widgets": {
    "text": {
      "type": "textarea",
      "placeholder": "Enter your prompt..."
    }
  }
}
```

Server 收到后：
1. 存入该 run 的 input 注册表
2. 推送给 Web → 前端渲染对应控件

### Input 操作下发

Web 前端用户操作控件后：

```
POST /api/runs/:id/input/:node_id/command
```

```json
{
  "output_id": "text",
  "value": "Tell me about quantum computing"
}
```

Server 收到后存入命令队列。

### Input 轮询

dm-input 节点轮询：

```
GET /api/runs/:id/input/:node_id/poll?since=42
```

```json
{
  "commands": [
    {"seq": 43, "output_id": "text", "value": "Tell me about quantum computing"}
  ]
}
```

### 产物文件访问

Web 前端渲染时拉取文件：

```
GET /api/runs/:id/out/*path
```

Server 从 `runs/:id/out/` 目录读取文件并返回，自动设置 `Content-Type`。

---

## Web 前端设计

### 页面结构

在 Run 详情页新增一个 "Interactive" 区域，动态渲染 display 和 input 节点：

```
┌─ Run: abc123 ─────────────────────────────────────────┐
│ ┌─ Header ─────────────────────────────────────────┐  │
│ │ Status: Running  [Graph] [YAML] [Transpiled]     │  │
│ └──────────────────────────────────────────────────┘  │
│                                                       │
│ ┌─ Display Panels ─────────────────────────────────┐  │
│ │ ┌─ Chat Log ──────┐  ┌─ Preview ──────────────┐ │  │
│ │ │ [10:30] Hello    │  │                        │ │  │
│ │ │ [10:31] World    │  │    [image preview]     │ │  │
│ │ │ [10:32] ...      │  │                        │ │  │
│ │ │ ▼ auto-scroll    │  │                        │ │  │
│ │ └─────────────────┘  └────────────────────────┘ │  │
│ └──────────────────────────────────────────────────┘  │
│                                                       │
│ ┌─ Input Controls ─────────────────────────────────┐  │
│ │ ┌─ Prompt Input ──────────────────────────────┐  │  │
│ │ │ [Enter your prompt...                    ] ↵ │  │  │
│ │ └─────────────────────────────────────────────┘  │  │
│ │ ┌─ Control Panel ─────────────────────────────┐  │  │
│ │ │ [Start Processing]   Volume: ───●─── 50     │  │  │
│ │ └─────────────────────────────────────────────┘  │  │
│ └──────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────┘
```

- Display panels 从上到下排列，每个 dm-display 节点一个面板
- Input controls 从上到下排列，每个 dm-input 节点一个控件组
- 多个同类节点自然堆叠，无需额外布局逻辑
- 全部动态渲染——没有 display/input 节点时这个区域不显示

### 组件映射

| 节点 | 前端组件 |
|------|---------|
| dm-display (text) | 滚动文本面板 (monospace, auto-scroll) |
| dm-display (json) | JSON tree viewer |
| dm-display (image) | 自动刷新图片 |
| dm-display (audio) | 音频播放器 |
| dm-display (video) | 视频播放器 |
| dm-input (textarea) | 多行文本框 + 发送按钮 |
| dm-input (button) | 按钮 |
| dm-input (slider) | Slider 控件 |
| dm-input (select) | 下拉选择框 |
| dm-input (switch) | 开关 |

---

## 典型用法示例

### 示例 1: LLM 对话

```yaml
nodes:
  - id: llm
    node: dm-qwen
    inputs:
      prompt: user-prompt/text
    outputs: [response]

  - id: log-response
    node: dm-log
    inputs:
      data: llm/response
    config:
      path: "chat.log"
      format: text

  - id: show
    node: dm-display
    inputs:
      path: log-response/path
    config:
      label: "LLM Response"
      render: text
      tail: true

  - id: user-prompt
    node: dm-input
    outputs: [text]
    config:
      label: "Your Prompt"
      widgets:
        text:
          type: textarea
          hotkey: "Enter"
```

### 示例 2: 摄像头监控 Dashboard

```yaml
nodes:
  - id: camera
    path: ./camera_node.py
    outputs: [frame]

  - id: save-frame
    node: dm-save
    inputs:
      data: camera/frame
    config:
      dir: "frames/"
      overwrite_latest: true
      extension: png

  - id: preview
    node: dm-display
    inputs:
      path: save-frame/path
    config:
      label: "Live Camera"
      render: image
```

### 示例 3: 纯控制台（无展示）

```yaml
nodes:
  - id: controls
    node: dm-input
    outputs: [start, stop, speed]
    config:
      label: "Robot Controls"
      widgets:
        start:
          type: button
          label: "Start"
          hotkey: "Space"
        stop:
          type: button
          label: "Emergency Stop"
          hotkey: "Escape"
        speed:
          type: slider
          min: 0
          max: 100
          default: 30

  - id: robot
    path: ./robot_controller.py
    inputs:
      start: controls/start
      stop: controls/stop
      speed: controls/speed
```

---

## 与现有 run_ws 的关系

现有的 `run_ws.rs` 负责 runtime monitoring（日志、metrics、状态）。交互族的通信是一个**独立的通道**：

| 通道 | 用途 | 方向 |
|------|------|------|
| `run_ws` | 运行时监控 (logs/metrics/status) | server → web（只读） |
| display API | 展示通知 + 文件内容 | node → server → web |
| input API | 用户操作 | web → server → node |

三个通道互不干扰。前端可以同时打开 RuntimeGraphView 看监控，同时在 Interactive 区域操作展示和输入。

---

## 实现顺序

| Step | 内容 | 依赖 |
|------|------|------|
| I1 | dm-server: `/api/runs/:id/out/*path` 文件服务 | 无 |
| I2 | dm-server: display 通知端点 + WS 推送 | I1 |
| I3 | dm-display 节点实现 | I2, 存储族节点 |
| I4 | dm-server: input 注册/命令/轮询端点 | 无 |
| I5 | dm-input 节点实现 | I4 |
| I6 | Web: Display 面板渲染组件 | I2 |
| I7 | Web: Input 控件渲染组件 | I4 |
| I8 | Web: Interactive 区域整合到 Run 详情页 | I6, I7 |

---

## 与架构原则的一致性

- ✅ **节点业务纯粹** — dm-display 只通知，dm-input 只采集
- ✅ **展示不碰 Arrow** — 展示的是已持久化的文件，不做序列化
- ✅ **节点不起服务器** — 节点是 dm-server 的 HTTP 客户端
- ✅ **dm-core 无关** — core 不知道这些节点的存在
- ✅ **平台无关** — 换一个 CLI 版的 display/input 实现，同一个 YAML 就变成命令行应用
