# 消息格式协议：Embed 渲染描述

> 消息格式协议 v1

## 概述

Dora Manager 的消息系统基于 `seq`、`from`、`tag`、`payload`、`timestamp` 五个核心字段。其中 `payload` 是一个自由 JSON 对象，由发送节点自行决定内容结构。为支持富格式消息渲染，我们引入可选字段 `payload.embed`——一个结构化的**渲染描述对象**。

**核心设计原则：** 所有现有代码无需改动。不加 `embed` 时行为与以往完全一致。加 `embed` 可获得卡片式富渲染。

## 消息结构

```json
{
  "seq": 42,
  "from": "ai-assistant",
  "tag": "text",
  "payload": {
    "content": "Hello world",
    "embed": { ... }
  },
  "timestamp": 1715000000000
}
```

`payload` 中可选的 `embed` 对象定义了消息在前端的渲染方式。

## Embed 字段参考

### 消息位置与外观

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `side` | `"left"` / `"right"` / `"center"` | `"left"` | 消息气泡位置 |
| `width` | `"full"` / `"compact"` | `"compact"` | 消息宽度 |

### 发送者信息

```json
"author": {
  "name": "AI Detector v2",
  "icon": "🤖",
  "url": "/nodes/detector"
}
```

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `author.name` | string | 是 | 发送者显示名称 |
| `author.icon` | string | 否 | 行内图标（emoji 或图标名） |
| `author.url` | string | 否 | 点击跳转链接 |

### 标题

```json
"title": "Detection Result",
"title_url": "https://..."
```

| 字段 | 类型 | 描述 |
|------|------|------|
| `title` | string | 卡片标题（有则渲染为卡片头部） |
| `title_url` | string | 标题可点击跳转链接 |

### 正文内容（Matrix 风格）

```json
"body": "Person detected at 0.95 confidence",
"body_formatted": "**Person** detected at *0.95* confidence",
"body_format": "markdown"
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `body` | string | — | 纯文本降级内容 |
| `body_formatted` | string | — | 富文本内容（可选） |
| `body_format` | `"plain"` / `"markdown"` | `"plain"` | 富文本格式类型 |

前端规则：`body_formatted` 存在时优先渲染，否则渲染 `body`。`body_format` 为 `"markdown"` 时对 `body_formatted` 做 markdown 渲染。

### 颜色侧边条

```json
"color": "green"
"color": 0x22c55e
```

| 值类型 | 示例 | 说明 |
|--------|------|------|
| 语义名称 | `"green"`, `"red"`, `"yellow"`, `"blue"`, `"purple"`, `"gray"`, `"orange"` | 预定义颜色 |
| 16 进制整数 | `0x22c55e` | RGB 颜色值 |
| 16 进制字符串 | `"#22c55e"` | CSS 风格 |

### 分组字段

```json
"fields": [
  { "name": "Class", "value": "person", "inline": true },
  { "name": "Confidence", "value": "0.95", "inline": true },
  { "name": "Bounding Box", "value": "x:120 y:80 w:200 h:300" }
]
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `fields[].name` | string | — | 字段名称 |
| `fields[].value` | string | — | 字段值 |
| `fields[].inline` | boolean | `false` | 是否同行排列 |

`inline: true` 的字段会尽可能在同一行并排显示。

### 媒体附件

```json
"media": {
  "url": "/api/runs/{id}/artifacts/snapshot.jpg",
  "type": "image",
  "width": 640,
  "height": 480,
  "alt": "Detection result screenshot",
  "caption": "Frame #142 — person detected"
}
```

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `media.url` | string | 是 | 媒体资源 URL |
| `media.type` | `"image"` / `"video"` / `"audio"` | 是 | 媒体类型 |
| `media.width` | integer | 否 | 显示宽度（像素） |
| `media.height` | integer | 否 | 显示高度（像素） |
| `media.alt` | string | 否 | 替代文本 |
| `media.caption` | string | 否 | 图片下方说明文字 |

### 缩略图

```json
"thumbnail": {
  "url": "/api/runs/{id}/artifacts/thumb.jpg",
  "width": 80,
  "height": 80
}
```

显示在消息右上角的小图。仅包含 `url` 为必填。

### 页脚

```json
"footer": {
  "text": "dora-yolo v0.3.1",
  "icon": "⚡"
}
```

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `footer.text` | string | 是 | 页脚文本 |
| `footer.icon` | string | 否 | 页脚图标 |

### 时间戳显示

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `timestamp` | `"relative"` / `"absolute"` / `"hidden"` | `"relative"` | 时间显示模式 |

### 操作按钮

```json
"actions": [
  { "label": "View Details", "url": "/runs/{id}/nodes/detector", "style": "link" },
  { "label": "Download", "url": "/api/artifacts/report.json", "style": "button" }
]
```

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `actions[].label` | string | 是 | 按钮文字 |
| `actions[].url` | string | 是 | 点击跳转链接 |
| `actions[].style` | `"link"` / `"button"` | 否 | 显示样式 |

### 状态指示

```json
"status": "running",
"progress": 0.75
```

| 字段 | 类型 | 描述 |
|------|------|------|
| `status` | `"pending"` / `"running"` / `"success"` / `"warning"` / `"error"` | 消息状态 |
| `progress` | float (0.0~1.0) | 进度百分比（仅 `status=running` 时有用） |

## 渲染优先级

前端按 embed 中存在哪些字段决定渲染形态：

| embed 中包含 | 渲染结果 |
|---|---|
| 无 embed | 降级到传统 tag+payload 渲染 |
| 仅有 `body` | 纯文本消息 |
| `body` + `body_formatted` | 富文本消息 |
| `color` | 带颜色侧边栏 |
| `author` | 发送者信息头部 |
| `title` | 可点击标题行 |
| `fields` | 结构化字段表格 |
| `media` | 媒体嵌入卡片 |
| `thumbnail` | 右上角缩略图 |
| `footer` | 底部信息行 |
| `actions` | 操作按钮组 |
| `status` + `progress` | 状态条/进度条 |
| `side` | 消息气泡对齐 |

多个字段同时存在时按固定布局排列：`author` > `title` > `body`/`fields` > `media` > `thumbnail` > `footer` > `actions`。

## SDK 使用

### Python SDK

```python
import dm

# 方式 1: 设置默认 embed 模板
msg = dm.Message()
msg.embed(
    author={"name": "AI Assistant", "icon": "🤖"},
    color="blue",
    footer={"text": "dora-yolo v0.3"},
)

# 后续所有 send 自动带上 embed
msg.send("text", {"content": "Hello"})
# → payload 中包含:
#   {content: "Hello", embed: {author: {name: "AI Assistant", icon: "🤖"}, color: 0x22c55e, ...}}

# 方式 2: 单条消息覆盖 embed
msg.send("text", {"content": "Warning!"}, embed={"color": "red"})

# 方式 3: 链式调用
msg = dm.Message().embed(author={"name": "Bot"}, color="green")

# 方式 4: 不传 embed（传统方式，完全向后兼容）
msg.send("text", {"content": "hello"})  # 没有 embed
```

### dm-display 节点使用 embed

```python
import dm

msg = dm.Message()
msg.send("text", {
    "content": "Detection result",
    "embed": {
        "author": {"name": "YOLO Detector", "icon": "🔍"},
        "body_formatted": "**Person** detected at *0.95* confidence",
        "body_format": "markdown",
        "color": "green",
        "fields": [
            {"name": "Class", "value": "person", "inline": True},
            {"name": "Confidence", "value": "0.95", "inline": True},
        ],
    }
})
```

### 前端 JS 消费端

前端 MessageItem.svelte 检查 `entry.payload.embed` 是否存在：
- 存在 → 使用 embed 渲染管线
- 不存在 → 降级到现有 tag+payload 渲染

## 向后兼容

所有现有代码无需修改。embed 是完全可选的。以下消息全部有效：

```python
# 旧式（无 embed）
msg.send("text", {"content": "hello"})

# 新式（完整 embed）
msg.send("text", {"content": "hello", "embed": {"body": "hello"}})

# 新式（纯 embed，忽略 payload 中的传统字段）
msg.send("text", {"anything": "goes", "embed": {"body": "hello"}})
```

## 相关阅读

- [交互系统架构：SDK 双端口模型与消息服务](22-jiao-hu-xi-tong-jia-gou-sdk-shuang-duan-kou-mo-xing-yu-xiao-xi-fu-wu) — 消息系统的整体架构
- [响应式控件（Widgets）](20-xiang-ying-shi-kong-jian-widgets-kong-jian-zhu-ce-biao-dong-tai-xuan-ran-yu-websocket-can-shu-zhu-ru) — 控件注册与前端渲染
