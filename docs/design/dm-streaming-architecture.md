# DM Streaming Architecture

> Status: design draft

这组文档定义 dora-manager 下一阶段的媒体流能力。

目标不是继续扩展 `dm-mjpeg`，而是为 `dm-server` 引入一个正式的媒体面能力，
让 live video / audio / point cloud / 3D 等 panel 都建立在统一架构上。

相关拆分文档：

- [`dm-server-mediamtx-integration.md`](./dm-server-mediamtx-integration.md)
- [`dm-screen-stream-node.md`](./dm-screen-stream-node.md)

---

## 1. 结论

推荐方案：

- `dm-server` 负责 **控制面**
- `MediaMTX` 负责 **媒体面**
- node 负责 **采集 / 编码 / 发布**
- web panel 负责 **订阅 stream metadata 并消费播放地址**

不推荐继续把 `MJPEG` 当主路线，也不推荐在 `dm-server` 内部手写 RTSP / HLS / WebRTC 服务。

---

## 2. 为什么不继续做 dm-mjpeg

`dm-mjpeg` 适合作为：

- 简单预览
- 调试链路
- 零依赖浏览器展示

但它不适合作为 streaming architecture 的基础设施，因为它缺少：

- 多协议能力
- 低延迟浏览器主通路
- 鉴权与会话编排
- 转发与桥接能力
- 录制 / 回放
- 后续音视频 / RTC 扩展空间

结论：

- `dm-mjpeg` 保留为调试节点或 fallback
- 正式 live streaming 架构不再围绕 MJPEG 设计

---

## 3. 分层边界

### 3.1 dm-core

负责：

- run lifecycle
- node runtime orchestration
- 环境变量注入

不负责：

- media server lifecycle
- stream protocol negotiation
- viewer URL generation

### 3.2 dm-server

负责：

- run-scoped stream registry
- stream metadata persistence / query
- media backend orchestration
- viewer URL generation
- 面向 web 的 stream API

不负责：

- 视频转码
- WebRTC/RTSP/HLS 实际分发
- 实时媒体帧搬运

### 3.3 MediaMTX

负责：

- ingest
- publish / subscribe
- RTSP / HLS / WebRTC / RTMP / SRT 等协议出口
- recording / playback（未来可选）

### 3.4 nodes

负责：

- 采集媒体源
- 编码或转封装
- 发布到 MediaMTX
- 向 `dm-server` 发 `stream` metadata message

### 3.5 web panels

负责：

- 订阅 `stream` snapshots
- 按 `kind` 选择合适的 panel renderer
- 根据 `dm-server` 返回的 viewer 信息连接外部媒体流

---

## 4. 核心设计原则

### 4.1 控制面与数据面分离

`messages` 负责元数据，不负责大流量媒体数据。

```text
node -> MediaMTX        (media data plane)
node -> dm-server       (stream metadata control plane)
web  -> dm-server API   (viewer resolution)
web  -> MediaMTX        (actual playback)
```

### 4.2 dm-server 编排，不嵌入

`MediaMTX` 被视为外部服务进程，而不是 Rust crate。

`dm-server` 用 Rust 负责：

- find / install binary
- generate config
- spawn / stop process
- health checks

而不是自己实现媒体协议栈。

### 4.3 panel 继续沿用现有抽象

未来新增的 `VideoPanel`、`AudioPanel`、`PointCloudPanel` 都是：

`stream snapshot renderer + external transport consumer`

不会破坏当前 panel system。

### 4.4 path 与 run 绑定

所有媒体流 path 都必须绑定 run，避免全局命名冲突：

```text
runs/{run_id}/{node_id}/{stream_name}
```

### 4.5 优先浏览器友好协议

建议：

- publish: `RTSP`（第一版）
- play primary: `WebRTC`
- play fallback: `HLS`
- debug only: `MJPEG`

---

## 5. 第一阶段范围

第一阶段只做最小闭环：

1. `dm-server` 集成 `MediaMTX`
2. 定义 `stream` message schema
3. 新增 run-scoped stream registry / viewer API
4. 新增 `dm-screen-capture` 和 `dm-stream-publish` 节点
5. 新增 `VideoPanel`

不在第一阶段做：

- TURN 自动部署
- 复杂 RTC 房间模型
- 录制回放 UI
- 点云 / 3D panel
- 多媒体服务后端切换

---

## 6. 目标能力

第一阶段完成后，理想链路：

```text
dm-screen-capture
  -> emit frame into Dora graph

dm-stream-publish
  -> publish RTSP to MediaMTX
  -> emit stream metadata to dm-server

dm-server
  -> store stream snapshot
  -> resolve viewer URLs

web VideoPanel
  -> query stream snapshot
  -> load WebRTC viewer
  -> fallback to HLS if needed
```

---

## 7. 后续扩展

这套架构建立后，可以自然扩展：

- `kind = "audio"` -> AudioPanel
- `kind = "pointcloud"` -> PointCloudPanel
- `kind = "scene3d"` -> Scene3DPanel
- recording/replay
- remote observer
- multi-stream layouts

核心不变：

- `dm-server` 仍是控制面
- `MediaMTX` 或其他 backend 仍是媒体面
