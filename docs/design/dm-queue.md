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

## 适用范围

dm-queue 是 **单输入流的缓冲/聚合节点**。它负责：
- 对 `data` 输入做 FIFO 缓冲
- 按容量、时长或控制信号触发 flush
- 在容量不足时提供背压、环形覆写或文件缓冲

它 **不负责**：
- 多路 stream join / merge
- 按 key 分组聚合
- 业务级窗口计算

如果上游存在多个 logical stream，必须在进入 dm-queue 前拆成多个节点实例，或在 metadata 中保证单调、无交叉的 stream 边界。

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

## 数据模型与时间语义

### 输入消息

`data` 输入上的每条消息由两部分组成：
- payload: Arrow-compatible 二进制数据块，由 dm-queue 原样缓存，不做业务解码
- metadata: 可选 JSON 对象，用于 stream 边界、时长和时间戳

支持的 metadata 字段：

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `stream` | `"start"` \| `"chunk"` \| `"end"` | 否 | 声明流边界；缺省视为普通 chunk |
| `stream_id` | string | 否 | 流唯一标识；存在时必须在同一 queue 实例内保持一致直到 flush |
| `timestamp_ns` | uint64 | 否 | 该 chunk 对应的事件时间 |
| `duration_ns` | uint64 | 否 | 该 chunk 覆盖的媒体/采样时长 |
| `content_type` | string | 否 | 透传给下游，便于消费方识别 |

### `max-size-time` 计算规则

`max-size-time` / `current-level-time` 使用以下优先级计算：

1. 若消息带 `duration_ns`，则累计 `duration_ns`
2. 否则若队列内相邻消息都带 `timestamp_ns`，则使用 `last.timestamp_ns - first.timestamp_ns`
3. 否则退化为 wall clock 停留时间：`now_monotonic_ns - first_enqueue_monotonic_ns`

实现要求：
- 同一个队列实例必须固定使用首次可用的时间模式，直到本次 flush 完成
- 若从步骤 2 或 3 退化到更低精度模式，必须发出一条 `buffering` 状态事件，`reason = "time_estimation_fallback"`

### 单流约束

dm-queue 默认只接受一个活跃流：
- 收到 `stream = "start"` 时，若当前队列为空，则进入 active stream 状态
- 若当前队列非空且 `stream_id` 与活跃流不同，节点必须先执行一次 flush，再接受新流
- 若无 `stream_id`，则所有消息都视为同一流

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
    - buffering
    - error
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

### 输入端口

| 端口 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `data` | data | 是 | 待缓冲消息流 |
| `control` | string or JSON | 否 | 控制命令 |

### `control` 协议

支持以下命令：

| 命令 | 说明 |
|---|---|
| `"flush"` | 立即 flush 当前队列；空队列时无操作 |
| `"stop"` | 拒绝后续 `data` 输入并 flush 当前队列 |
| `"reset"` | 丢弃当前队列内容并清空状态，不产出 `flushed` |

不支持 `"start"` 命令。流开始由 `data.metadata.stream = "start"` 表示，避免双重状态机。

### 输出端口

| 端口 | 类型 | 说明 |
|---|---|---|
| `flushed` | data | 一次 flush 的聚合结果 |
| `buffering` | JSON | 水位、背压、时间退化等状态事件 |
| `error` | JSON | 超时、协议错误、文件缓冲错误等不可恢复事件 |

### `flushed` 消息格式

`flushed` 的 payload 为按到达顺序拼接的 chunk 列表或文件引用，不做业务合并：

```json
{
  "storage": "memory",
  "items": [
    { "index": 0, "bytes": 1024, "metadata": { "timestamp_ns": 1 } },
    { "index": 1, "bytes": 2048, "metadata": { "timestamp_ns": 2 } }
  ],
  "total_items": 2,
  "total_bytes": 3072,
  "total_duration_ns": 1000000,
  "stream_id": "tts-42",
  "flush_reason": "signal"
}
```

当启用 `temp-template` 且数据已落盘时：

```json
{
  "storage": "file",
  "path": "/tmp/dm-queue-abcd12.bin",
  "meta_path": "/tmp/dm-queue-abcd12.meta.json",
  "total_items": 128,
  "total_bytes": 52428800,
  "total_duration_ns": 30000000000,
  "stream_id": "camera-1",
  "flush_reason": "full"
}
```

### `buffering` 事件格式

```json
{
  "event": "high_watermark",
  "current_level_buffers": 95,
  "current_level_bytes": 1992294,
  "current_level_time": 1800000000,
  "mode": "backpressure",
  "reason": null
}
```

`event` 可选值：
- `high_watermark`
- `low_watermark`
- `backpressure_started`
- `backpressure_released`
- `ring_overwrite`
- `time_estimation_fallback`

### `error` 事件格式

```json
{
  "code": "max_block_time_exceeded",
  "message": "producer blocked for longer than max-block-time",
  "recoverable": false
}
```

`code` 可选值：
- `max_block_time_exceeded`
- `invalid_stream_transition`
- `file_spool_io_error`
- `flush_emit_error`

## 边界情况

| 场景 | 方案 | 参考 |
|---|---|---|
| 信号丢失 | `flush-timeout` 兜底 | Flink idle timeout |
| 溢出 | 背压 → 阻塞 `max-block-time` → 报错 | Kafka `max.block.ms` |
| 超大单条 | 直接透传 | Kafka bypass |
| 并发流 | 仅支持单活跃流；新流到达前先 flush 旧流 | GStreamer flush event |
| 崩溃恢复 | 仅保证落盘文件可诊断，不保证自动恢复消费 | Flink checkpoint |
| 内存不足 | `temp-template` 文件缓冲 | GStreamer `temp-template` |

补充约束：
- 超大单条 bypass 仅适用于标准队列模式；若启用 ring buffer，超大单条必须写入 `error`
- `flush-timeout` 基于“最后一次成功入队时间”计算，而不是基于 stream start 时间
- `use-buffering = false` 时不发送 `buffering` 事件，但仍维护内部水位

## 状态机

```text
idle
  -> receiving          on first data
receiving
  -> blocked            on capacity reached in standard mode
  -> receiving          on ring overwrite in ring mode
  -> flushing           on signal/full/timeout/control.flush
blocked
  -> receiving          on capacity released before max-block-time
  -> error              on max-block-time exceeded
flushing
  -> idle               on flush success and queue empty
  -> error              on emit or spool failure
error
  -> idle               on control.reset
```

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

## 实现要求

- 节点必须保证单 producer / single queue state 的顺序一致性，不允许并发 flush 打乱 FIFO
- `flushed` 中的 item metadata 必须完整保留上游 metadata，除非字段与 dm-queue 保留字段冲突
- 文件缓冲模式下，`meta.json` 至少包含 item 边界、原始 metadata、总字节数和 flush_reason
- 若上游/下游阻塞导致 flush 无法送出，节点必须通过 `error` 报警，而不是静默丢弃
