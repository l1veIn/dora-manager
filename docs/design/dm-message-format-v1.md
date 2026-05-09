# dm-message-format v1 — 消息渲染格式设计

## 背景

当前 Message 系统的 payload 是完全自由的 `serde_json::Value`。节点之间靠约定往 payload 里塞 `{content, label, kind, file}` 等字段，前端 MessageItem.svelte 靠硬编码的 `payload.content`、`payload.file` 等来渲染。这有几个问题：

1. **没有标准渲染描述** — 消息位置（左/右）、样式、发送者信息无法表达
2. **前端和节点之间没有契约** — 一个节点塞了 `content` 但另一个塞了 `text` 就无法统一渲染
3. **无法表达富格式** — 多字段卡片、颜色标签、行内操作按钮等

## 设计原则

1. **完全向后兼容** — 现有 `msg.send("text", {"content": "hello"})` 代码不需要改
2. **协议层加可选字段** — payload 中增加可选的 `embed` 渲染描述，不改变现有核心数据结构
3. **前端优先降级** — 没有 embed 时使用现有的 tag+payload 渲染逻辑
4. **借鉴成熟标准** — 以 Discord Embed 为结构参考，Matrix m.text 为内容格式参考

## Schema

### 消息体（服务端存储格式）

```rust
pub struct Message {
    pub seq: i64,
    pub from: String,
    pub tag: String,
    pub payload: Value,      // 兼容现有自由格式
    pub timestamp: i64,
}
```

`payload` 中可选的 `embed` 字段：

```json
{
  // 传统模式（向后兼容，无 embed 时降级）
  "content": "Hello world",
  "label": "Detection Result",
  "file": "relative/path/to/image.png",
  "kind": "inline|file",

  // 新增：可选渲染描述
  "embed": {
    // --- 消息位置与外观 ---
    "side": "left|right|center",          // 消息气泡位置（默认 left）
    "width": "full|compact",               // 消息宽度（默认 compact）

    // --- 发送者信息 ---
    "author": {
      "name": "AI Detector v2",            // 发送者显示名
      "icon": "🤖",                        // 行内图标（emoji）
      "url": "/nodes/detector"             // 点击跳转链接（可选）
    },

    // --- 标题（可选，有则显示为卡片头部） ---
    "title": "Detection Result",
    "title_url": "https://...",

    // --- 正文内容（Matrix 风格：body + formatted_body 分离） ---
    "body": "Person detected at 0.95 confidence",     // 纯文本 fallback
    "body_formatted": "**Person** detected at *0.95* confidence",  // 富文本（markdown）
    "body_format": "markdown",                         // 取值: "plain" | "markdown"（默认 plain）

    // --- 颜色侧边条 ---
    "color": 0x00ff00,                     // 16 进制整数 RGB，类似 Discord
    // 或语义色:
    "color": "green|red|yellow|blue|purple|gray",

    // --- 分组字段（类似 Discord fields） ---
    "fields": [
      { "name": "Class", "value": "person", "inline": true },
      { "name": "Confidence", "value": "0.95", "inline": true },
      { "name": "Bounding Box", "value": "x:120 y:80 w:200 h:300", "inline": false }
    ],

    // --- 图片/视频/音频附件 ---
    "media": {
      "url": "/api/runs/{id}/artifacts/snapshot.jpg",
      "type": "image|video|audio",
      "width": 640,
      "height": 480,
      "alt": "Detection result screenshot",
      "caption": "Frame #142 — person detected"
    },

    // --- 缩略图（小图，显示在右上角） ---
    "thumbnail": {
      "url": "/api/runs/{id}/artifacts/thumb.jpg",
      "width": 80,
      "height": 80
    },

    // --- 页脚 ---
    "footer": {
      "text": "dora-yolo v0.3.1",
      "icon": "⚡"
    },

    // --- 时间戳显示 ---
    "timestamp": "relative|absolute|hidden",  // 显示模式（默认 relative）

    // --- 操作按钮 ---
    "actions": [
      { "label": "View Details", "url": "/runs/{run_id}/nodes/detector", "style": "link" },
      { "label": "Download", "url": "/api/artifacts/report.json", "style": "button" }
    ],

    // --- 状态指示 ---
    "status": "pending|running|success|warning|error",
    "progress": 0.75  // 0.0~1.0，仅 status=running 时
  }
}
```

### 所有字段均为可选

`embed` 对象中的每个字段都是可选的。前端按存在性逐级渲染：

| embed 中存在的字段 | 渲染效果 |
|---|---|
| 空 embed / 无 embed | 降级到现有 tag+payload 渲染 |
| 仅有 `body` | 纯文本消息 |
| `body` + `body_formatted` | 富文本消息（支持 markdown） |
| `body` + `body_formatted` + `color` | 带颜色侧边栏的富文本消息 |
| `author` + `body` + `footer` | 完整卡片消息 |
| `fields` | 结构化数据表格 |
| `media` | 图片/视频/音频嵌入 |
| `actions` | 底部操作按钮 |
| `status` | 状态指示器或进度条 |
| `title` | 可点击的标题行 |
| `side` | 消息气泡位置（left/right/center） |

## 向后兼容示例

### 现有代码（不改）

```python
# dm-display 发送文本
msg.send("text", {
    "label": "Temperature",
    "kind": "inline",
    "content": "Current temp: 24°C"
})
```

前端渲染：不检查 embed，降级到 `payload.content` + tag="text"，渲染为纯文本气泡。

### 新代码（使用 embed）

```python
# dm-display 发送富格式消息
msg.send("text", {
    "content": "Person detected!",
    "embed": {
        "body_formatted": "**Person** detected at *0.95* confidence",
        "body_format": "markdown",
        "color": "green",
        "author": {"name": "YOLO Detector", "icon": "🔍"},
        "fields": [
            {"name": "Class", "value": "person", "inline": True},
            {"name": "Confidence", "value": "0.95", "inline": True}
        ],
        "side": "left"
    }
})
```

### SDK 封装（推荐使用方式）

```python
msg.embed(
    body="**Person** detected at *0.95* confidence",
    body_format="markdown",
    color="green",
    author={"name": "YOLO Detector", "icon": "🔍"},
    fields=[
        ("Class", "person", True),
        ("Confidence", "0.95", True),
    ],
)
```

## 前端渲染规则

### MessageItem.svelte 渲染管线

```
entry.payload.embed 存在？
  ├── 是 → 使用 embed 渲染：卡片式布局
  │   ├── side=right → 气泡居右，发送者信息可选隐藏
  │   ├── color → 左侧色条
  │   ├── author → 头部发送者信息
  │   ├── title → 可点击标题
  │   ├── body / body_formatted → 正文（支持 markdown）
  │   ├── fields → 字段表格（inline 支持自动换行）
  │   ├── media → 媒体嵌入
  │   ├── thumbnail → 右上角缩略图
  │   ├── footer → 底部信息
  │   ├── actions → 按钮组
  │   └── status → 状态条/进度条
  │
  └── 否 → 降级到现有渲染
      ├── entry.from === "web" → UserInputMessageItem
      ├── entry.tag === "widgets" → WidgetRegistrationItem
      └── 其他 → 按 tag 渲染 content/file
```

## 实现范围

| 层级 | 改动 | 风险 |
|---|---|---|
| 服务端 types.rs | 无改动（payload 已经是 JSON，embed 是 payload 内的可选字段） | 无 |
| SDK _message.py | 新增 `msg.embed()` 方法，封装 embed 构建 | 低（新增方法，不删不改） |
| 前端 MessageItem.svelte | 重写渲染逻辑，支持 embed 分支 | 中等（需要确保降级路径完全不变） |
| 前端 MessagePanel.svelte | 可能需要调整卡片容器样式 | 低 |
| 文档 wiki | 新增消息格式文档 | — |

## Non-Goals

- 本设计不涉及消息路由、消息查询协议的改进
- 不涉及 WebSocket 推送协议的变化
- 不涉及消息格式在服务端的验证（payload 仍然是自由 JSON）
- 不涉及 `message_snapshots` 表的变化（embed 存在 payload 中即可）
