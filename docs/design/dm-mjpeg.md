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

## 适用范围

dm-mjpeg 是 **预览节点**，不是录制、归档或转码节点。它负责：
- 接收视频帧
- 按需缩放并编码为 JPEG
- 对外提供 `/stream` 实时预览端点

它 **不负责**：
- 回放历史帧
- 音视频同步
- 鉴权、转码编排、长连接管理平台化

## 配置

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `port` | uint | `4567` | HTTP 监听端口 |
| `host` | string | `"127.0.0.1"` | 监听地址，默认仅本机可访问 |
| `quality` | uint (1-100) | `80` | JPEG 压缩质量 |
| `max-fps` | uint | `30` | 最大帧率（丢弃多余帧） |
| `width` | uint | `0` (原始) | 缩放宽度（0=不缩放） |
| `height` | uint | `0` (原始) | 缩放高度（0=不缩放） |
| `input-format` | `jpeg` \| `rgb8` \| `rgba8` \| `yuv420p` | `jpeg` | 输入帧像素格式 |
| `drop-if-no-client` | bool | `true` | 无客户端连接时仅保留最新帧 |
| `allow-origin` | string | `null` | 可选 CORS 头，仅用于外部前端接入 |

## 输入协议

### 输入端口

| 端口 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `frame` | data | 是 | 视频帧输入 |

### 帧 schema

`frame` payload 是单帧图像数据，metadata 必须至少包含：

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `format` | string | 否 | 缺省时使用 `input-format` 配置 |
| `width` | uint | `jpeg` 以外必填 | 帧宽 |
| `height` | uint | `jpeg` 以外必填 | 帧高 |
| `stride` | uint | 否 | 行跨度；缺省按紧密排列计算 |
| `timestamp_ns` | uint64 | 否 | 源时间戳，用于帧率限制与调试 |
| `content_type` | string | 否 | `image/jpeg` 或原始像素 MIME |

支持的 payload 格式：
- `jpeg`: payload 为完整 JPEG 字节流；dm-mjpeg 可直接复用，必要时仅做尺寸重编码
- `rgb8`: 按 `width * height * 3` 排列的 RGB 数据
- `rgba8`: 按 `width * height * 4` 排列的 RGBA 数据
- `yuv420p`: 平面 YUV420 数据；必须提供 `width` / `height`

不支持的 `format` 必须返回错误事件，不能静默跳过。

## 输出与 HTTP 接口

dm-mjpeg 不向 dora 下游输出视频数据；它的输出是 HTTP 服务和可选状态日志。

### HTTP 端点

| 路径 | 方法 | 说明 |
|---|---|---|
| `/stream` | `GET` | MJPEG multipart 实时流 |
| `/healthz` | `GET` | 进程存活检查，返回 `200 OK` |
| `/snapshot.jpg` | `GET` | 返回最近一帧 JPEG；无帧时返回 `503` |

### `/stream` 行为

- 响应头必须包含 `Content-Type: multipart/x-mixed-replace; boundary=frame`
- 若当前尚无可用帧，连接保持打开，直到首帧到达；超过 30s 仍无帧则返回 `503`
- 每个客户端只读取“最新已编码帧”，不保证逐帧送达
- `max-fps` 作用于编码更新频率，而不是每个客户端独立节流
- 当客户端消费速度慢于生产速度时，服务器必须跳过旧帧，只发送最新帧

### 错误模型

建议通过节点日志和统一错误事件上报，错误码如下：

| code | 说明 |
|---|---|
| `unsupported_input_format` | 输入格式未实现 |
| `invalid_frame_shape` | 宽高、stride、payload 长度不匹配 |
| `jpeg_encode_failed` | JPEG 编码失败 |
| `http_bind_failed` | 端口监听失败 |
| `client_write_failed` | 向客户端写流失败 |

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
      host: 127.0.0.1
      port: 4567
      input-format: jpeg
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
      - buffering
      - error
    config:
      ring-buffer-max-size: 52428800
      max-size-time: 30000000000

  # MJPEG preview stays on the live feed.
  # dm-queue is used for recording buffer, not preview transport.
  - id: preview
    node: dm-mjpeg
    inputs:
      frame: camera/frame
    config:
      host: 127.0.0.1
      port: 4567
      input-format: jpeg
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
    <img src={streamUrl} alt="Live Preview" crossorigin="anonymous" />
    <button on:click={() => open = false}>Close</button>
  </div>
{/if}
```

若 dm web UI 与节点不在同源，需要显式配置 `allow-origin`，否则默认不返回 CORS 头。

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
  → validate frame metadata against input-format
  → decode / reinterpret pixels
  → resize (if configured)
  → encode JPEG (quality param)
  → replace shared latest-frame slot
  → HTTP handler snapshots latest-frame and writes multipart response
```

### 依赖

| crate | 用途 |
|---|---|
| `dora-node-api` | dora 节点接口 |
| `image` | 图像缩放 + JPEG 编码 |
| `axum` 或 `tiny_http` | HTTP 服务（MJPEG multipart） |
| `tokio` | async runtime |

> `axum` 与 dm-server 技术栈一致；如果求极简可用 `tiny_http`（零依赖 HTTP）。

## 与 dm-queue 的协作边界

- `dm-mjpeg` 只消费“当前预览帧”，不读取 `dm-queue/buffering`
- 若业务需要“录制最近 30 秒”，应由 `dm-queue` 负责缓冲与 flush，`dm-mjpeg` 继续直接订阅 live feed
- 若未来需要“从 ring buffer 回放预览”，应新增独立节点，避免把 `dm-mjpeg` 扩展成双职责组件

## 实现要求

- 默认监听 `127.0.0.1`；只有用户显式配置 `0.0.0.0` 时才允许对局域网暴露
- 节点必须维护单个原子“最新 JPEG 帧”槽位，禁止为慢客户端积压无界队列
- `drop-if-no-client = true` 时，无客户端连接不保留历史帧，只更新最新槽位
- `/snapshot.jpg` 与 `/stream` 必须共享同一最新帧缓存，避免重复编码
