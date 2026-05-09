# Widget Manager — 控件管理 API 设计

## 背景

目前控件的注册和输入接收是分离的两条路径：

- **注册**：节点调用 `msg.send("widgets", {label, widgets, ...})`，把 JSON 丢进消息服务的 snapshot
- **输入接收**：节点轮询 `msg.get(tag="input")`，自己过滤 `payload.to` 或 `payload.widget_key`

这有两个问题：
1. **注册和接收没有统一抽象** — 开发者需要手动拼 JSON、写过滤逻辑
2. **widget_key 没有协议支持** — 哪些 key 被注册了、每个 key 的配置是什么，全靠节点自己管理

## Widget Manager 设计

在 `dm.Message` 上新增 `.widgets` 属性，返回 `WidgetManager` 实例：

```python
msg = dm.Message()

# 列出所有已注册的 widget
all_widgets = msg.widgets.list()
# → [Widget(key="slider-temp", type="slider", label="Temperature", config={...}), ...]

# 注册/更新一个 widget
slider = msg.widgets.register(
    key="slider-temp",       # widget_key：唯一标识，用于前端路由和节点订阅
    type="slider",
    label="Temperature (°C)",
    config={
        "min": -20,
        "max": 50,
        "step": 1,
        "default": 20,
    },
)

# 更新已有 widget 的配置
msg.widgets.update(key="slider-temp", disabled=True)

# 删除一个 widget
msg.widgets.remove(key="slider-temp")

# 订阅某个 widget 的输入
for event in msg.widgets.subscribe("slider-temp"):
    value = event["value"]
    process(value)
```

## Widget 协议

### 注册（服务端存储格式）

Widget 注册仍然是走 `tag="widgets"` 的 snapshot，但 payload 中增加 `widget_key` 字段：

```json
{
  "tag": "widgets",
  "payload": {
    "label": "SDK Demo",
    "widget_key": "sdk-demo-input",
    "widgets": {
      "value": {
        "type": "input",
        "label": "Text to reverse",
        "default": "",
        "placeholder": "Type something..."
      }
    }
  }
}
```

### 输入消息（前端发送格式）

前端发送 input 时，payload 中带 `widget_key` 替代 `to`：

```json
{
  "tag": "input",
  "payload": {
    "widget_key": "sdk-demo-input",
    "value": "hello world",
    "output_id": "value"
  }
}
```

### Subscribe（节点接收过滤）

节点通过 `widget_key` 过滤，无需关心 `from` 或 `to`：

```python
# 内部实现
def subscribe(self, widget_key):
    while True:
        messages = self.msg.get(tag="input", after_seq=LAST_SEQ)
        for m in messages:
            if m["payload"].get("widget_key") == widget_key:
                yield m["payload"]
```

## Widget 动态管理能力

| 操作 | 方法 | 效果 |
|------|------|------|
| 注册 | `.register(key, type, label, config)` | 新增 widget，前端自动显示 |
| 更新配置 | `.update(key, **config)` | 修改 label、placeholder 等，前端自动刷新 |
| 禁用/启用 | `.update(key, disabled=True)` | 前端控件变灰不可操作 |
| 隐藏/显示 | `.update(key, hidden=True)` | 控件从前端消失 |
| 修改样式 | `.update(key, color="blue")` | 控件卡片颜色变化 |
| 删除 | `.remove(key)` | 前端控件移除 |
| 枚举 | `.list()` | 返回所有已注册 widget |
| 订阅输入 | `.subscribe(key)` | 接收该 widget 的用户输入 |

## 前端 InputPanel 改动

1. 读取 widget snapshot 时，优先使用 `payload.widget_key` 作为消息路由标识
2. 发送 input 时，`payload.widget_key` 替代 `payload.to`
3. 支持 `disabled` 和 `hidden` 状态

## 向后兼容

现有 `payload.to` 的 input 消息仍被支持（节点需要继续监听旧格式直到所有节点迁移）。

## 实现计划

| 层级 | 改动 |
|------|------|
| SDK _message.py | 新增 `WidgetManager` 类 + `Message.widgets` 属性 |
| SDK _message.py | `WidgetManager.register()` 封装 `tag="widgets"` 的 send |
| SDK _message.py | `WidgetManager.subscribe()` 封装轮询 + widget_key 过滤 |
| 前端 InputPanel | 读取 `payload.widget_key`，发送时用 widget_key |
| 前端 InputPanel | 支持 disabled/hidden 状态 |
| SDK demo | 改用 `msg.widgets.register()` + `.subscribe()` |
| Wiki 文档 | 更新 28-message-protocol.md |
