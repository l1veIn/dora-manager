# dm-server + MediaMTX Integration

> Status: design draft

本文关注实现层问题：

- `dm-server` 如何跨平台集成 `MediaMTX`
- 为什么这里不应该找 Rust crate 替代它
- `dm-server` 需要新增哪些模块和 API

---

## 1. 关键判断

`MediaMTX` 应该被视为：

- **外部服务二进制**
- 由 `dm-server` 编排
- 而不是嵌入式 Rust 库

原因：

- 它的官方分发形态是 standalone binary / Docker image
- 实际能力是完整 media server，而不是 SDK
- 跨平台最稳定的接入方式就是“下载并管理二进制”

因此，`dm-server` 需要的是一个 **media runtime wrapper**，而不是一个自研媒体栈。

---

## 2. 跨平台策略

目标平台：

- macOS
- Linux
- Windows

推荐分两阶段。

### 2.1 阶段一：显式路径优先

先支持以下配置方式：

- `DM_MEDIAMTX_PATH`
- `dm-server` settings 中配置 `mediamtx.path`

行为：

- 若配置存在，则直接使用该二进制
- `DM_MEDIAMTX_PATH` 优先于 config

这一层已经实现。

### 2.2 阶段二：自动下载

当前实现：

- 根据 `os + arch + version` 下载官方 release 资产
- 保存到本地 cache
- 若 cache 已存在则直接复用

当前未实现：

- checksum 校验

建议缓存目录：

```text
<dm-home>/bin/mediamtx/<version>/<platform>/mediamtx
```

例如：

```text
~/.dora-manager/bin/mediamtx/v1.11.1/darwin-arm64/mediamtx
```

不建议把 Docker 作为默认路径，因为桌面用户的部署依赖太重。

---

## 3. dm-server 内部模块建议

当前代码里先以内聚为主，落在：

```text
crates/dm-server/src/services/media.rs
```

后续如果逻辑继续增长，再拆成 `runtime / github / archive / health` 子模块。

### 3.1 runtime.rs

负责统一生命周期：

- resolve binary
- ensure installed
- spawn process
- stop process
- restart process
- collect logs

核心结构建议：

```rust
pub struct MediaRuntime {
    backend: MediaBackend,
    process: Option<Child>,
    state: MediaRuntimeState,
}
```

### 3.2 mediamtx.rs

封装 MediaMTX 特定行为：

- config file generation
- launch arguments
- api endpoint assumptions
- health check logic

### 3.3 config.rs

负责从 `dm-server` config 转成 MediaMTX config。

例如：

- listen ports
- WebRTC/HLS 开关
- path auth
- recording policy

### 3.4 health.rs

负责：

- process liveness
- TCP/HTTP health check
- metrics endpoint checks

---

## 4. 媒体运行时模型

### 4.1 启动时机

第一版建议：

- `dm-server` 启动时初始化 `MediaRuntime`
- 若 MediaMTX 不可用，不阻塞 `dm-server` 主功能
- 但将 streaming 功能标记为 unavailable

这样：

- interaction / panel / run 管理不受影响
- streaming 功能可单独降级

### 4.2 失败语义

Media backend 启动失败时：

- `dm-server` 继续运行
- `GET /api/media/status` 返回 `error` / `unconfigured`
- stream viewer API 对未就绪流返回空 `viewer`

### 4.3 单实例还是每 run 一实例

第一版建议：

- **一个 dm-server 对应一个 MediaMTX 进程**

不建议 per-run 启一个 MediaMTX：

- 成本太高
- 端口管理复杂
- WebRTC/HLS 配置重复

run 维度隔离依赖 path 命名和鉴权，而不是进程隔离。

---

## 5. 路径命名

建议强制 run-scoped：

```text
runs/{run_id}/{node_id}/{stream_name}
```

示例：

```text
runs/092a3eed/camera_front/main
runs/092a3eed/screen-recorder/main
```

好处：

- 不冲突
- 易于调试
- 易于 viewer URL 生成
- 易于 run cleanup

---

## 6. stream registry

`dm-server` 需要新增 stream registry，但它只是元数据 registry。

建议新增：

- `stream_messages` 直接复用现有 `messages`，tag=`stream`
- `stream_snapshots` 可以直接从 `messages/snapshots` 读取

也就是说，第一版不需要新 DB 表。

实现方式：

- node emit `tag = "stream"`
- server 按现有 messages 体系落库
- stream service 从 snapshot 里筛出 `tag = "stream"`

只有当将来需要更复杂的 stream 索引时，再考虑独立表。

---

## 7. viewer API

建议新增 API：

### 7.1 查询 run 下全部 stream

```text
GET /api/runs/{id}/streams
```

返回：

```json
{
  "streams": [
    {
      "stream_id": "screen-recorder/main",
      "from": "screen-recorder",
      "kind": "video",
      "label": "Desktop",
      "live": true,
      "status": "ready",
      "viewer": {
        "preferred": "webrtc",
        "webrtc_url": "https://host/api/media/webrtc/runs/123/screen-recorder/main",
        "hls_url": "https://host/api/media/hls/runs/123/screen-recorder/main/index.m3u8"
      }
    }
  ]
}
```

### 7.2 查询单条 stream

```text
GET /api/runs/{id}/streams/{stream_id}
```

### 7.3 媒体服务状态

```text
GET /api/media/status
```

返回：

```json
{
  "backend": "mediamtx",
  "status": "ready",
  "version": "v1.x.x"
}
```

---

## 8. viewer URL 生成原则

前端不直接拼 MediaMTX 原生 URL。

应由 `dm-server` 统一生成 viewer 信息，原因：

- 避免前端耦合后端部署细节
- 便于未来切换 backend
- 可在 server 端统一做鉴权、代理、签名

因此 stream payload 中建议存：

- `path`
- `transport.play`

而最终 URL 由 `dm-server` 计算。

---

## 9. 配置建议

建议 `dm-server` 增加配置段：

```toml
[media]
backend = "mediamtx"
enabled = true

[media.mediamtx]
path = "/path/to/mediamtx"
auto_download = false
version = "v1.11.1"
api_port = 9997
rtsp_port = 8554
hls_port = 8888
webrtc_port = 8889
```

阶段一先支持：

- `enabled`
- `path`
- ports

阶段二再支持：

- `auto_download`
- `version`
- checksum policy

---

## 10. 运行与清理

### 启动

`dm-server`:

1. resolve MediaMTX binary
2. write runtime config
3. spawn process
4. wait for health ready

### 运行中

- node publish 到 path
- node emit `stream` metadata
- web 通过 viewer API 获取播放地址

### 停止

- `dm-server` shutdown 时终止 MediaMTX 子进程

### run cleanup

run 结束后不需要关闭 MediaMTX。

只需要：

- 清理 run 对应的 stream metadata
- 可选清理 path-specific recording artifacts

---

## 11. 第一阶段不做的内容

- 自动下载器
- 多 backend 抽象实现
- TURN 自动发现/部署
- stream ACL 精细权限
- 录制管理 UI

---

## 12. 实施建议

推荐按下面顺序：

1. 增加 `media` 模块和 status API
2. 先支持显式 `MediaMTX` 路径
3. 跑通进程托管与 health check
4. 增加 stream viewer API
5. 再接 `VideoPanel` 和 stream node
