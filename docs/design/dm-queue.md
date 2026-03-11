# dm-queue 设计文档

## 定位

**dm-queue 是 GStreamer `queue2` 的 Rust 实现，适配 dora 数据流框架。**

queue2 是经过 20+ 年验证的流控原语，dm-queue 完全复刻其设计模式和属性命名。
它处理的是离散的 Arrow 数据项（FIFO），与 queue2 的职责完全对齐：
**queue2 = "GstBuffer 的 queue"，dm-queue = "Arrow chunk 的 queue"。**

### 解决的问题

dm 数据流中缺少通用的流控节点。常见问题包括：
- 流式输出（TTS 音频）产生大量独立 chunk，需要聚合为完整 asset
- 监控视频需要滑动窗口缓冲，仅保留最近 N 秒
- 高频传感器数据需要批量聚合后再处理

## 设计原则

对齐 GStreamer queue2，补充 Kafka / Flink 经验：

1. **多维限制，取先到者**：buffers + bytes + time 三维并存
2. **背压优先于丢弃**：满了先阻塞上游推送线程
3. **参数驱动行为**：无 mode 开关，`ring-buffer-max-size > 0` 启用环形覆写
4. **水位线通知**：`use-buffering` 启用后根据 watermark 发送状态事件
5. **文件缓冲**：`temp-template` 非空时自动落盘
6. **超时兜底**：`flush-timeout` 防止数据永远滞留
7. **溢出三级降级**：背压 → 阻塞等待 → 超时报错
8. **超大数据 bypass**：单条超 `max-size-bytes` 直接透传

## 属性（Properties）

### 复刻 queue2

| 属性 | 类型 | 默认值 | 读写 | 说明 |
|---|---|---|---|---|
| `max-size-buffers` | uint | `100` | RW | 最大队列条数（0=禁用） |
| `max-size-bytes` | uint | `2097152`(2MB) | RW | 最大字节数（0=禁用） |
| `max-size-time` | uint64 (ns) | `2000000000`(2s) | RW | 最大时长（0=禁用） |
| `ring-buffer-max-size` | uint64 | `0` | RW | 0=标准队列；>0=环形覆写 |
| `use-buffering` | bool | `false` | RW | 启用水位线事件 |
| `high-watermark` | float | `0.99` | RW | 高水位线 |
| `low-watermark` | float | `0.01` | RW | 低水位线 |
| `temp-template` | string | `null` | RW | 文件缓冲模板路径（含 XXXXXX） |
| `temp-remove` | bool | `true` | RW | 结束时删除临时文件 |

### 只读监控

| 属性 | 类型 | 说明 |
|---|---|---|
| `current-level-buffers` | uint | 当前条数 |
| `current-level-bytes` | uint | 当前字节数 |
| `current-level-time` | uint64 | 当前数据时长 |
| `avg-in-rate` | int64 | 平均入队速率 (bytes/s) |

### dm 扩展

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `flush-on` | `signal` \| `full` | `signal` | flush 触发条件 |
| `flush-timeout` | uint64 (ns) | `0` | 空闲超时自动 flush |
| `max-block-time` | uint64 (ns) | `10000000000`(10s) | 背压阻塞上限 |

## YAML 配置示例

### TTS 音频聚合

```yaml
- id: tts-queue
  node: dm-queue
  inputs:
    data: dora-kokoro-tts/audio
  outputs:
    - flushed
  config:
    max-size-bytes: 52428800        # 50MB
    flush-on: signal                # wait for { "stream": "end" }
    flush-timeout: 30000000000      # 30s fallback
```

### 监控视频缓冲（环形）

```yaml
- id: cam-queue
  node: dm-queue
  inputs:
    data: camera/frame
    control: panel/record
  outputs:
    - flushed
    - overflow
  config:
    ring-buffer-max-size: 52428800  # 50MB ring
    max-size-time: 30000000000      # 30s window
    use-buffering: true
```

### 通用 Batch

```yaml
- id: batch-queue
  node: dm-queue
  inputs:
    data: sensor/reading
  outputs:
    - flushed
  config:
    max-size-buffers: 100
    flush-on: full
    flush-timeout: 5000000000       # 5s idle timeout
```

## 信号协议

- **上游 → queue**：metadata `{ "stream": "start" }` / `{ "stream": "end" }`
- **control 输入**：`"flush"` / `"start"` / `"stop"`
- **queue → 下游**：`flushed`（聚合数据）、`overflow`（水位线事件 + 丢弃通知）

## 边界情况

| 场景 | 方案 | 参考 |
|---|---|---|
| 信号丢失 | `flush-timeout` 兜底 | Flink idle timeout |
| 溢出 | 背压 → 阻塞 `max-block-time` → 报错 | Kafka `max.block.ms` |
| 超大单条 | 直接透传 | Kafka bypass |
| 并发流 | 自动 flush 前一个流 | GStreamer flush event |
| 崩溃恢复 | 定期写 `meta.json` 快照 | Flink checkpoint |
| 内存不足 | `temp-template` 文件缓冲 | GStreamer `temp-template` |

## 架构实现

### 位置与安装

独立 Rust 节点，位于 `nodes/dm-queue/`：

```
nodes/dm-queue/
├── Cargo.toml
├── src/
│   ├── main.rs          # dora node entry
│   ├── model.rs         # Properties struct
│   └── queue.rs         # VecDeque + ringbuf logic
├── dm.json
└── README.md
```

安装方式：`dm node install dm-queue`（执行 `cargo install`）。
不内置 dm-core —— dm-queue 是通用流控节点，dora 原版也可使用。

### 依赖

| crate | 用途 |
|---|---|
| `dora-node-api` | dora 节点标准接口 |
| `std::collections::VecDeque` | 标准队列 |
| `ringbuf` | 环形队列 |
| `bytes` | 零拷贝数据 |
| `tempfile` | 文件缓冲 |
