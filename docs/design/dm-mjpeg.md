# dm-mjpeg 设计文档

## 定位

dm-mjpeg 是一个 **sink 适配器节点**，将 dora 数据流中的视频帧暴露为 MJPEG over HTTP 端点，
供前端（dm web UI 弹窗）或外部工具（浏览器、VLC）实时预览。

与 dm-queue 是同级别的独立节点，不内置 dm-core。

### 同级节点

| 节点 | 输入 | 输出 | 说明 |
|---|---|---|---|
| `dm-mjpeg` | 视频帧 | HTTP 端点 | 浏览器 `<img>` 实时预览 |
| `dm-ndi` (未来) | 视频帧 | NDI 流 | 专业视频制作 |
| `dm-rtmp` (未来) | 视频帧 | RTMP 推流 | 直播/远程 |
| `dm-virtual-cam` (未来) | 视频帧 | 虚拟摄像头 | OBS/Zoom |

## 工作原理

### MJPEG over HTTP

MJPEG（Motion JPEG）是最简单的视频流协议——逐帧推送 JPEG 图片：

```
HTTP/1.1 200 OK
Content-Type: multipart/x-mixed-replace; boundary=frame

--frame
Content-Type: image/jpeg
Content-Length: 12345

<JPEG bytes>
--frame
Content-Type: image/jpeg
Content-Length: 12346

<JPEG bytes>
...
```

浏览器原生支持，一个 `<img>` 标签即可渲染：

```html
<img src="http://localhost:4567/stream" />
```

### 为什么选 MJPEG

| 方案 | 前端复杂度 | 后端复杂度 | 延迟 |
|---|---|---|---|
| **MJPEG over HTTP** | **零**（img 标签） | **低**（50 行） | ~1帧 |
| WebSocket + Canvas | 中（JS 解码渲染） | 中 | ~1帧 |
| WebRTC | 高 | 高 | 最低 |
| HLS/DASH | 低 | 高（分段编码） | 2-10s |

MJPEG 是复杂度最低、可靠性最高的方案。无音频、无播放控制，但对"实时预览"场景足够。

## 配置

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `port` | uint | `4567` | HTTP 监听端口 |
| `host` | string | `"0.0.0.0"` | 监听地址 |
| `quality` | uint (1-100) | `80` | JPEG 压缩质量 |
| `max-fps` | uint | `30` | 最大帧率（丢弃多余帧） |
| `width` | uint | `0` (原始) | 缩放宽度（0=不缩放） |
| `height` | uint | `0` (原始) | 缩放高度（0=不缩放） |

## YAML 配置示例

### 基础用法

```yaml
nodes:
  - id: camera
    path: camera-capture
    outputs:
      - frame

  - id: preview
    node: dm-mjpeg
    inputs:
      frame: camera/frame
    config:
      port: 4567
      quality: 75
      max-fps: 15
```

访问 `http://localhost:4567/stream` 即可实时预览。

### 搭配 dm-queue

```yaml
nodes:
  - id: camera
    path: camera-capture
    outputs:
      - frame

  # Ring buffer keeps latest 30s
  - id: cam-queue
    node: dm-queue
    inputs:
      data: camera/frame
    outputs:
      - flushed
      - overflow
    config:
      ring-buffer-max-size: 52428800
      max-size-time: 30000000000

  # MJPEG preview (reads latest frame from queue overflow)
  - id: preview
    node: dm-mjpeg
    inputs:
      frame: camera/frame         # direct feed for live preview
    config:
      port: 4567
      max-fps: 15

  # Panel receives flushed recordings
  - id: panel
    node: dm-panel
    inputs:
      recording: cam-queue/flushed
    outputs:
      - record
```

### dm web UI 集成

前端预览弹窗实现极其简单：

```svelte
<script>
  let streamUrl = "http://localhost:4567/stream";
  let open = false;
</script>

{#if open}
  <div class="preview-modal">
    <img src={streamUrl} alt="Live Preview" />
    <button on:click={() => open = false}>Close</button>
  </div>
{/if}
```

## 架构实现

### 位置与安装

```
nodes/dm-mjpeg/
├── Cargo.toml
├── src/
│   ├── main.rs          # dora node entry + HTTP server
│   └── encoder.rs       # frame → JPEG encoding
├── dm.json
└── README.md
```

安装：`dm node install dm-mjpeg`（`cargo install`）。

### 内部流程

```
dora input (Arrow frame)
  → decode raw pixels (if needed)
  → resize (if configured)
  → encode JPEG (quality param)
  → push to shared latest-frame slot
  → HTTP handler reads slot, writes multipart response
```

### 依赖

| crate | 用途 |
|---|---|
| `dora-node-api` | dora 节点接口 |
| `image` | 图像缩放 + JPEG 编码 |
| `axum` 或 `tiny_http` | HTTP 服务（MJPEG multipart） |
| `tokio` | async runtime |

> `axum` 与 dm-server 技术栈一致；如果求极简可用 `tiny_http`（零依赖 HTTP）。
