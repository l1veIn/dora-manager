# dm-queue 设计文档

## 定位

**dm-queue 是 GStreamer `queue2` 的 Rust 实现，适配 dora 数据流框架。**

GStreamer queue2 是经过 20+ 年验证的流控原语，dm-queue 完全复刻其设计模式和属性命名，
让有 GStreamer 经验的用户可以零学习成本上手。

### 解决的问题

Panel 消息流中，流式输出（TTS 音频、监控视频）会产生大量独立 block，淹没消息列表。
dm-queue 作为中间节点聚合流式数据，Panel 直接收到完整 asset：

```
tts-engine/speech → dm-queue → dm-panel（收到完整音频，问题消失）
```

### 为什么叫 queue 而不是 buffer

dora 节点之间传递的是离散的 Arrow 数据项，不是连续字节流。
dm-queue 排列这些离散数据项（FIFO），与 GStreamer queue2 的职责完全对齐：
**queue2 = "GstBuffer 的 queue"，dm-queue = "Arrow chunk 的 queue"。**

## 设计原则

完全对齐 GStreamer queue2：

1. **多维限制，取先到者**：buffers + bytes + time 三维并存
2. **背压优先于丢弃**：满了先阻塞上游推送线程
3. **参数驱动行为**：无 mode 开关，`ring-buffer-max-size > 0` 自动启用环形覆写
4. **水位线通知**：`use-buffering` 启用后，根据 high/low watermark 发送缓冲状态事件
5. **文件缓冲**：设置 `temp-template` 后自动落盘，内存限制仅用于统计

额外补充（来自 Kafka / Flink 经验）：

6. **超时必须有**：`flush-timeout` 防止数据永远滞留
7. **溢出三级降级**：背压 → 阻塞等待 → 超时报错
8. **超大数据 bypass**：单条超 `max-size-bytes` 直接透传

## 属性（Properties）

### 复刻 queue2 的属性

| 属性 | 类型 | 默认值 | 读写 | 说明 |
|---|---|---|---|---|
| `max-size-buffers` | uint | `100` | RW | 最大队列条数（0=禁用） |
| `max-size-bytes` | uint | `2097152`(2MB) | RW | 最大字节数（0=禁用） |
| `max-size-time` | uint64 (ns) | `2000000000`(2s) | RW | 最大缓冲时长（0=禁用） |
| `ring-buffer-max-size` | uint64 | `0` | RW | 环形队列大小。0=标准队列；>0=环形覆写 |
| `use-buffering` | bool | `false` | RW | 启用后根据水位线通过 `overflow` 输出发送状态事件 |
| `high-watermark` | float | `0.99` | RW | 高水位线（缓冲完成阈值） |
| `low-watermark` | float | `0.01` | RW | 低水位线（重新缓冲阈值） |
| `temp-template` | string | `null` | RW | 临时文件模板路径，含 XXXXXX。非空时启用文件缓冲 |
| `temp-remove` | bool | `true` | RW | run 结束时是否删除临时文件 |

### 只读监控属性

| 属性 | 类型 | 说明 |
|---|---|---|
| `current-level-buffers` | uint | 当前队列中的条数 |
| `current-level-bytes` | uint | 当前队列中的字节数 |
| `current-level-time` | uint64 | 当前队列中的数据时长 |
| `avg-in-rate` | int64 | 平均入队速率（bytes/s） |

### dm 扩展属性

queue2 不具备但 dora 场景需要的属性：

| 属性 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `flush-on` | `signal` \| `full` | `signal` | flush 触发条件。`signal`=等上游 metadata；`full`=满即冲 |
| `flush-timeout` | uint64 (ns) | `0` (禁用) | 空闲超时自动 flush，防数据滞留（**生产环境建议必填**） |
| `max-block-time` | uint64 (ns) | `10000000000`(10s) | 背压阻塞上限，超时报错（参考 Kafka `max.block.ms`） |

## YAML 配置示例

### TTS 音频聚合

```yaml
nodes:
  - id: tts-queue
    node: dm-queue
    inputs:
      data: dora-kokoro-tts/audio
    outputs:
      - flushed
    config:
      max-size-bytes: 52428800      # 50MB safety cap
      flush-on: signal              # wait for metadata { "stream": "end" }
      flush-timeout: 30000000000    # 30s fallback

  - id: panel
    node: dm-panel
    inputs:
      audio_tts: tts-queue/flushed
```

### 监控视频缓冲

```yaml
nodes:
  - id: cam-queue
    node: dm-queue
    inputs:
      data: camera/frame
      control: panel/record
    outputs:
      - flushed
      - overflow
    config:
      ring-buffer-max-size: 52428800  # 50MB ring buffer
      max-size-time: 30000000000      # 30s window
      use-buffering: true             # emit watermark events via overflow
      temp-template: /tmp/dm-queue-XXXXXX
```

### 通用 Batch

```yaml
nodes:
  - id: batch-queue
    node: dm-queue
    inputs:
      data: sensor/reading
    outputs:
      - flushed
    config:
      max-size-buffers: 100
      flush-on: full
      flush-timeout: 5000000000     # 5s idle timeout
```

## 信号协议

- **上游 → queue**：metadata `{ "stream": "start" }` / `{ "stream": "end" }`
- **control 输入**：`"flush"` / `"start"` / `"stop"` 字符串指令
- **queue → 下游**：
  - `flushed`：聚合后的完整数据
  - `overflow`：水位线事件（`warning` / `resume`）+ 丢弃通知（ring 模式）

## 边界情况处理

| 场景 | 方案 | 参考 |
|---|---|---|
| 信号丢失 | `flush-timeout` 兜底自动 flush | Flink idle timeout |
| 溢出 | 背压 → 阻塞 `max-block-time` → 报错并强制 flush | Kafka `max.block.ms` |
| 超大单条数据 | 直接透传到 `flushed`，不进队列 | Kafka bypass |
| 并发流 | 自动 flush 前一个流，开始新流 | GStreamer flush event |
| 崩溃恢复 | 定期写 `meta.json` 快照，重启尝试恢复 | Flink checkpoint |
| 内存不足 | 设置 `temp-template` 启用文件缓冲 | GStreamer `temp-template` |

## 架构实现

### 代码位置

嵌入 `dm-core`，与 Panel 同级：

```
crates/dm-core/src/runs/
├── panel/           # existing, ~640 lines
└── queue/           # new, ~300-400 lines
    ├── mod.rs       # serve entry (dm queue serve --run-id X --node-id Y)
    ├── model.rs     # Properties struct (mirrors queue2 properties)
    └── queue.rs     # core logic: VecDeque (standard) + ringbuf (ring)
```

### Transpile 集成

```rust
const RESERVED_NODE_IDS: &[&str] = &["dm-panel", "dm-queue"];
```

新增 `inject_queue` pass，展开为 `dm queue serve --run-id {run_id} --node-id {node_id}`。

### 存储

```
runs/{run_id}/queues/{node_id}/
├── meta.json        # checkpoint snapshot
├── temp/            # temp file buffering (auto-cleanup on run end)
└── assets/          # flushed outputs (persisted)
```

### 依赖

| crate | 用途 |
|---|---|
| `std::collections::VecDeque` | 标准队列（标准库自带） |
| `ringbuf` | 环形队列 SPSC 无锁实现 |
| `bytes` | 零拷贝数据管理 |
| `tempfile` | `temp-template` 文件缓冲 |
