# Screen Capture / Stream Publish Nodes & Stream Protocol

> Status: design draft

本文定义：

- `stream` message schema
- `dm-screen-capture` / `dm-stream-publish` 节点
- `VideoPanel` 第一版行为

---

## 1. stream message schema

推荐新增统一 tag：

```text
tag = "stream"
```

它只承载媒体元数据，不承载媒体字节。

### 1.1 payload v1

```json
{
  "kind": "video",
  "stream_id": "screen-recorder/main",
  "label": "Desktop",
  "path": "runs/123/screen-recorder/main",
  "live": true,
  "codec": "h264",
  "width": 1920,
  "height": 1080,
  "fps": 15,
  "transport": {
    "publish": "rtsp",
    "play": ["webrtc", "hls"]
  }
}
```

### 1.2 字段说明

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `kind` | string | 是 | `video` / `audio` / `pointcloud` / `scene3d` |
| `stream_id` | string | 是 | node 内部稳定 stream id |
| `label` | string | 否 | 前端展示标题 |
| `path` | string | 是 | MediaMTX path |
| `live` | bool | 是 | 是否 live stream |
| `codec` | string | 否 | 例如 `h264` |
| `width` | uint | 否 | 视频宽 |
| `height` | uint | 否 | 视频高 |
| `fps` | uint | 否 | 帧率 |
| `transport.publish` | string | 是 | `rtsp` / `whip` 等 |
| `transport.play` | array | 是 | 可播放协议列表 |

### 1.3 约束

- 第一版 `kind` 只实现 `video`
- `path` 必须是 run-scoped
- `stream_id` 在单个 node 内稳定

---

## 2. 节点拆分定位

新的推荐拆分是两个节点：

- `dm-screen-capture`
  - 采集桌面或窗口
  - 输出 Arrow 帧数据到 Dora 图
- `dm-stream-publish`
  - 接收上游帧流
  - 发布到 MediaMTX
  - 向 `dm-server` 注册 stream metadata

这样 media backend 是一个标准下游，而不是采集节点内部耦合的副作用。

---

## 3. 节点职责

### dm-screen-capture 负责

- screen capture
- 输出 `frame` / `meta`

### dm-screen-capture 不负责

- MediaMTX
- `stream` message
- 浏览器播放

### dm-stream-publish 负责

- 接收 `frame`
- ffmpeg/gstreamer 调用
- 发布 RTSP
- emit `stream` message

### dm-stream-publish 不负责

- HLS/WebRTC 服务本身
- 录制管理平台逻辑
- 原始画面采集

---

## 4. 节点配置建议

### dm-screen-capture config

```json
{
  "capture_target": "desktop",
  "width": 1280,
  "height": 720,
  "mode": "repeat",
  "interval_sec": 1,
  "output_format": "png"
}
```

### dm-stream-publish interaction

```json
{
  "interaction": {
    "emit": ["stream"]
  }
}
```

### dm-stream-publish config

```json
{
  "fps": 15,
  "width": 1280,
  "height": 720,
  "codec": "h264",
  "stream_name": "main"
}
```

---

## 5. 节点运行模型

启动时：

1. `dm-screen-capture` 周期性或按触发输出 `frame`
2. `dm-stream-publish` 读取 `DM_RUN_ID`
3. `dm-stream-publish` 读取 `DM_NODE_ID`
4. `dm-stream-publish` 读取 `DM_SERVER_URL`
5. `dm-stream-publish` 组合 MediaMTX publish path
6. `dm-stream-publish` 启动 ffmpeg publish
7. `dm-stream-publish` emit `stream` metadata

示例 path：

```text
runs/{run_id}/{node_id}/main
```

示例 emit：

```python
emit("stream", {
    "kind": "video",
    "stream_id": f"{node_id}/main",
    "label": "Desktop",
    "path": f"runs/{run_id}/{node_id}/main",
    "live": True,
    "codec": "h264",
    "width": 1280,
    "height": 720,
    "fps": 15,
    "transport": {
        "publish": "rtsp",
        "play": ["webrtc", "hls"]
    }
})
```

---

## 6. 平台采集实现建议

第一版不做统一底层 capture 库，直接用外部工具。

### macOS

优先：

- `ffmpeg` + `avfoundation`

### Linux

优先：

- `ffmpeg` + `x11grab`

后续：

- `pipewire`

### Windows

优先：

- `ffmpeg` + `gdigrab`

这意味着第一版两个节点都更像 Python wrapper / Rust wrapper，内部启动系统 ffmpeg。

而不是自带复杂原生采集实现。

---

## 7. ffmpeg publish 示例

方向性示例：

```bash
ffmpeg \
  -f avfoundation -framerate 15 -i "1" \
  -vcodec libx264 -preset veryfast -tune zerolatency \
  -pix_fmt yuv420p \
  -f rtsp rtsp://127.0.0.1:8554/runs/<run_id>/<node_id>/main
```

实际命令需按平台调整。

---

## 8. VideoPanel 设计

`VideoPanel` 是专用 panel：

- sourceMode: `snapshot`
- subscribed tags: `stream`
- payload filter: `kind === "video"`

### 8.1 行为

- 显示符合条件的最新视频 stream snapshot
- 通过 `GET /api/runs/{id}/streams` 获取 viewer 信息
- 优先使用 `webrtc_url`
- fallback `hls_url`

### 8.2 UI 最小元素

- 流标题
- 来源 node
- 状态
- video player 容器

第一版不需要：

- 复杂播放器控制条
- 多轨道切换
- bitrate selector

---

## 9. 与当前 panel system 的关系

它符合现有抽象：

- `MessagePanel`：history renderer
- `InputPanel`：widgets snapshot renderer
- `ChartPanel`：chart snapshot renderer
- `VideoPanel`：stream snapshot renderer + external transport

因此不需要改 Workspace 基础架构。

---

## 10. 第一阶段实施清单

### server

- 新增 `stream` viewer API
- 在 stream snapshot 上建立服务层

### node

- 新增 `dm-screen-capture`
- 新增 `dm-stream-publish`
- 跑通 capture -> publish -> RTSP

### web

- 新增 `VideoPanel`
- 根据 stream snapshot 渲染 live stream

---

## 11. 后续扩展

这个协议后续可直接扩展到：

- 麦克风节点
- 摄像头节点
- 录屏 + 音频
- 点云流节点
- 3D scene 流节点

不需要重做 server 基础模型，只需要：

- 扩展 `kind`
- 扩展 panel renderer
- 扩展 node publish 实现
