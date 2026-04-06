# DM Readiness And Logic Nodes

> Status: design draft

这份文档定义一组轻量的“检查节点”和“逻辑节点”，用于把环境依赖、
平台服务可用性和业务前置条件显式表达进 dataflow。

它和 streaming 架构不是替代关系，而是补充关系。

相关文档：

- [`dm-streaming-architecture.md`](./dm-streaming-architecture.md)
- [`dm-server-mediamtx-integration.md`](./dm-server-mediamtx-integration.md)

---

## 1. 问题

目前很多 run 能否成功，依赖一些 dataflow 外部条件：

- `ffmpeg` 是否已安装
- 屏幕录制权限是否已授予
- 模型文件是否已下载
- `dm-server` media backend 是否 ready
- 麦克风 / 摄像头权限是否可用

这些条件现在分散在：

- node 内部失败
- `dm-server` 启动前校验
- web 设置页提示

问题是 dataflow 本身看不到这些依赖，也无法显式组合这些条件。

---

## 2. 结论

建议新增两类节点：

1. 检查节点

负责判断某个依赖是否满足，输出标准化 readiness 信号。

2. 逻辑节点

负责组合多个 readiness 信号，并控制后续数据是否放行。

同时保留系统级 service 的现有设计：

- `MediaMTX` 继续作为 `dm-server` 的 service
- `ffmpeg` 继续作为 node 内部使用的本地工具

也就是：

- `MediaMTX` 不节点化
- 但 `MediaMTX readiness` 可以节点化

---

## 3. 为什么不把 MediaMTX 纯节点化

`MediaMTX` 和 `ffmpeg` 都是外部二进制，但系统角色不同。

### 3.1 MediaMTX 更像平台服务

特征：

- 生命周期通常跨多个 run
- 需要端口管理
- 需要 viewer URL 生成
- 需要 settings/status UI
- 需要自动下载、版本管理、健康检查
- 需要和 web panel 深度集成

因此它更适合：

- `dm-server` service
- `web settings` 管理
- `stream registry` 消费

### 3.2 ffmpeg 更像本地工具

特征：

- 短生命周期
- 一般只由某个 node 进程内部调用
- 不需要全局 viewer API
- 不需要跨 run 共享实例

因此它更适合：

- node implementation dependency
- 再配一个 readiness check node

---

## 4. 分层策略

建议保留三层：

### 4.1 平台服务层

- `dm-server`
- `MediaMTX`
- 将来的 TURN / model service / vector DB

### 4.2 检查节点层

负责把系统条件转换成标准化布尔信号。

例如：

- `dm-check-ffmpeg`
- `dm-check-screen-permission`
- `dm-check-media-backend`
- `dm-check-file`
- `dm-check-model-ready`

### 4.3 逻辑编排层

负责把 readiness 信号组合成更高层条件。

例如：

- `dm-and`
- `dm-or`
- `dm-not`
- `dm-gate`

---

## 5. 节点协议建议

### 5.1 检查节点统一输出

所有检查节点尽量统一输出：

- `ok`
  - `bool`
- `details`
  - `json` / `text`
- 可选 `reason`
  - `text`

推荐 `details` 至少包含：

- `kind`
- `ready`
- `checked_at`
- `source`
- 失败时的错误信息

示例：

```json
{
  "kind": "ffmpeg",
  "ready": true,
  "version": "7.1.1",
  "path": "/opt/homebrew/bin/ffmpeg",
  "checked_at": 1775472000
}
```

### 5.2 逻辑节点统一输入

逻辑节点尽量只消费 `bool`，不理解领域细节。

例如 `dm-and`：

- `in1`, `in2`, `in3`, ...
- 输出 `ok`

例如 `dm-gate`：

- `enabled: bool`
- `value: any`
- `meta: any` 可选

输出：

- `value`
- `meta`

只有在 `enabled == true` 时才透传。

---

## 6. 推荐第一批节点

### 6.1 `dm-check-ffmpeg`

用途：

- 检查本机是否可执行 `ffmpeg`
- 返回版本与路径

输出：

- `ok`
- `details`

### 6.2 `dm-check-media-backend`

用途：

- 检查 `dm-server /api/media/status`
- 判断 media backend 是否 ready

输出：

- `ok`
- `details`

### 6.3 `dm-check-screen-permission`

用途：

- 检查当前平台是否具备桌面采集权限

输出：

- `ok`
- `details`

说明：

这个节点的实现需要按平台分别处理。
第一版可以允许“能力有限但可诊断”。

### 6.4 `dm-and`

用途：

- 聚合多个 readiness 条件

行为：

- 全部输入为 true 时输出 true
- 任一 false 时输出 false

### 6.5 `dm-gate`

用途：

- 用布尔条件控制任意数据流是否继续向下游传递

行为：

- `enabled == true` 时透传
- 否则阻断

---

## 7. 典型 dataflow

### 7.1 Streaming readiness flow

```text
dm-check-ffmpeg
dm-check-screen-permission
dm-check-media-backend
        -> dm-and
        -> dm-gate
        -> dm-screen-capture
        -> dm-stream-publish
```

### 7.2 Model readiness flow

```text
dm-check-file(model.bin)
dm-check-gpu
dm-check-network
        -> dm-and
        -> model node
```

---

## 8. 和现有系统的关系

### 8.1 不替代 server-side validation

检查节点不能替代 `dm-server` 的硬校验。

例如：

- dataflow 使用 media-capable node，但 backend 未 ready

这种情况 `dm-server` 仍然应该拒绝 run。

节点化检查的价值是：

- 让 dataflow 自己显式表达条件
- 让测试流更完整
- 让运行时行为更可观察

### 8.2 不替代 web settings / install UX

web settings 仍然需要负责：

- 安装 MediaMTX
- 查看 media 状态
- 展示错误引导

检查节点解决的是 graph 内编排问题，不是平台管理 UI。

---

## 9. 第一阶段建议

建议先实现最小集：

1. `dm-check-media-backend`
2. `dm-check-ffmpeg`
3. `dm-and`
4. `dm-gate`

这样就足以先把 streaming test flow 做成显式 readiness flow。

`dm-check-screen-permission` 可以第二阶段再补，因为它的平台差异最大。

---

## 10. 后续扩展

这套设计可以自然扩展到更多 readiness 条件：

- 文件存在
- 目录可写
- API key 已配置
- 模型已下载
- GPU 可用
- 端口未占用
- 远端服务可连接

逻辑节点仍然保持通用，不引入领域耦合。
