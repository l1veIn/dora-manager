# Run-Level Arrow Recording & Replay

> Status: **Design document (future phase)**

## 概述

Run-level Recording 是一个**平台级能力**，不是节点。它在 dora 数据流的传输层捕获所有 Arrow 事件，以 Parquet 格式持久化为有时间顺序的记录文件，支持后续回放。

类比：这就是数据流的"行车记录仪"——忠实记录每一个事件的发生时间、来源、目标和完整数据。

```
运行中的 dataflow
  node-A ──Arrow──→ node-B ──Arrow──→ node-C
           ↓                  ↓
      ┌────────────────────────────┐
      │  Recording Layer (拦截层)  │
      │  捕获所有 Arrow 事件       │
      │  写入 parquet              │
      └────────────┬───────────────┘
                   ↓
         runs/:id/record/
         ├── 000001.parquet
         ├── 000002.parquet
         └── manifest.json
```

## 这不是节点

Recording 是 **dm-core / dm-server 的平台能力**：

| 存储族节点 (dm-log/save/recorder) | Run-level Recording |
|--------------------------------|---------------------|
| 用户在 YAML 中显式编排 | 用户在 Run 配置中开关 |
| 只保存订阅的输入端口数据 | 保存所有节点间的全量数据 |
| 序列化为业务格式 (text/image/wav) | 原样保存 Arrow 二进制 + 元数据 |
| 节点级关注点 | 实例级关注点 |

---

## 录制

### 开启方式

```bash
# CLI 启动时开启
dm start dataflow.yml --record

# API 启动时开启
POST /api/dataflows/:id/start
{ "record": true }

# 运行中动态开启/关闭
POST /api/runs/:id/record/start
POST /api/runs/:id/record/stop
```

### 录制数据模型

每个 Arrow 事件被捕获为一条记录，Schema 如下：

| Column | Type | Description |
|--------|------|-------------|
| `seq` | uint64 | 全局单调递增序号 |
| `timestamp` | timestamp[us] | 事件捕获时的精确时间戳 |
| `source_node` | utf8 | 发送节点 ID |
| `source_output` | utf8 | 发送端口名称 |
| `target_node` | utf8 | 接收节点 ID |
| `target_input` | utf8 | 接收端口名称 |
| `data_type` | utf8 | Arrow 数据类型描述 |
| `data_size` | uint64 | 数据大小 (bytes) |
| `data` | large_binary | Arrow 序列化后的原始字节 |
| `metadata` | utf8 (json) | 事件元数据（dm_type 等）JSON 序列化 |

### 文件分片策略

Parquet 文件按固定大小或时间间隔分片：

```
runs/:id/record/
├── manifest.json           # 录制元数据
├── 000001.parquet          # 第一个分片
├── 000002.parquet          # 第二个分片
└── ...
```

分片规则（可配置，默认值）：
- `max_shard_size`: `128 MB` — 单个 parquet 文件最大大小
- `max_shard_duration`: `5 min` — 单个 parquet 文件最大时间跨度

两个条件哪个先满足就触发分片。

### manifest.json

```json
{
  "version": 1,
  "run_id": "abc123",
  "dataflow_name": "my-flow",
  "recording_started_at": "2026-04-01T14:00:00Z",
  "recording_stopped_at": "2026-04-01T14:30:00Z",
  "total_events": 158432,
  "total_bytes": 536870912,
  "shards": [
    {
      "file": "000001.parquet",
      "seq_range": [1, 52144],
      "time_range": ["2026-04-01T14:00:00Z", "2026-04-01T14:05:00Z"],
      "size_bytes": 134217728,
      "event_count": 52144
    },
    {
      "file": "000002.parquet",
      "seq_range": [52145, 106288],
      "time_range": ["2026-04-01T14:05:00Z", "2026-04-01T14:10:00Z"],
      "size_bytes": 134217728,
      "event_count": 54143
    }
  ],
  "nodes": ["node-a", "node-b", "node-c"],
  "edges": [
    {"source": "node-a/output1", "target": "node-b/input1"},
    {"source": "node-b/output1", "target": "node-c/input1"}
  ],
  "config": {
    "max_shard_size": "128 MB",
    "max_shard_duration": "5 min"
  }
}
```

### 录制实现要点

录制层需要在 dora 的事件传递链中拦截 Arrow 事件。两种可能的实现路径：

**路径 A：日志解析（低侵入）**

不修改 dora 运行时，从节点日志和 dora 的事件追踪中提取信息。与现有 `run_ws.rs` 的日志追踪机制类似。

- 优点：零侵入，不影响数据流性能
- 缺点：无法完整获取 Arrow 二进制数据，只能捕获元数据

**路径 B：代理节点（中等侵入）**

在 transpile 阶段为每条 edge 插入一个透明的录制代理节点：

```
原始: node-A ──→ node-B
录制: node-A ──→ recorder-proxy ──→ node-B
                      ↓
                 parquet writer
```

- 优点：完整获取 Arrow 数据，精确时间戳
- 缺点：增加数据流延迟，transpiler 复杂度增加

**路径 C：dora 事件钩子（需要 dora 支持）**

如果 dora 运行时提供事件订阅/钩子 API，可以在不修改数据流拓扑的情况下旁路捕获所有事件。

- 优点：零延迟影响，完整数据，架构最干净
- 缺点：需要 dora 上游支持

> **建议**：先评估 dora 是否有事件订阅能力。如果没有，从路径 B 开始，transpiler 已经有插入节点的能力（之前的 Panel 注入就是类似模式）。

---

## 回放

### 回放模式

```bash
# CLI
dm replay <run-id> [options]

# API
POST /api/runs/:id/replay
{
  "speed": 1.0,
  "start_seq": 0,
  "end_seq": null,
  "skip_storage": true
}
```

### 回放参数

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `speed` | float | `1.0` | 回放速度倍率。`2.0` = 2 倍速，`0.5` = 半速 |
| `start_seq` | uint64 | `0` | 从第几条事件开始回放 |
| `end_seq` | uint64/null | `null` | 回放到第几条事件停止，null = 至末尾 |
| `start_time` | timestamp/null | `null` | 按时间戳开始（替代 start_seq） |
| `end_time` | timestamp/null | `null` | 按时间戳结束 |
| `skip_storage` | bool | `true` | 是否跳过存储族节点（避免重复持久化） |
| `node_filter` | string[]/null | `null` | 只回放指定节点的事件 |

### 回放机制

1. 读取 `manifest.json` 确定分片范围
2. 按 `seq` 顺序依次读取 parquet 分片
3. 对每条记录：
   - 计算与上一条事件的时间差 `Δt`
   - 按 `speed` 倍率等待 `Δt / speed`
   - 将 Arrow 数据注入到 `target_node` 的 `target_input` 端口
4. 如果 `skip_storage = true`：注入环境变量 `DM_REPLAY=true`，存储族节点据此跳过写入

### 回放与存储族的协调

```python
# 存储族节点统一检查
if os.environ.get("DM_REPLAY") == "true":
    print(f"[dm-{name}] Replay mode: skipping persistence", file=sys.stderr)
    continue
```

当 `skip_storage = false` 时，存储族节点正常工作——这适用于"用旧输入重新跑流程看新结果"的场景。

### 回放的两种呈现方式

**方式 A：后端回放 + 前端观察**

实际启动 dataflow，用录制的事件替代真实输入。节点真实执行。用户通过 RuntimeGraphView 观察。

- 适用场景：验证修改后的节点行为
- 代价：需要启动完整运行时

**方式 B：纯前端回放（轻量）**

不启动 dataflow。前端直接读取 parquet，按时间线渲染事件到 RuntimeGraphView 上，只展示数据流动动画和 NodeInspector 中的事件内容。

- 适用场景：纯 debug 回看
- 代价：只能看，不能改
- 优点：秒开，无需后端配合

> **建议**：先实现方式 B（纯前端回放），因为它不需要 dora 运行时参与，实现简单。方式 A 作为进阶能力后续支持。

---

## 存储空间管理

### 录制空间估算

| 数据流类型 | 事件频率 | 每事件大小 | 每小时 |
|-----------|---------|-----------|-------|
| 纯文本 LLM 对话 | ~1/s | ~1 KB | ~3.6 MB |
| 音频流 16kHz | ~100/s | ~320 B | ~115 MB |
| 视频流 30fps | ~30/s | ~200 KB | ~21 GB |
| 传感器数据 | ~1000/s | ~100 B | ~360 MB |

### Run 级存储限制

```json
// 可以在 run start 时配置
{
  "record": true,
  "record_config": {
    "max_total_size": "10 GB",
    "max_duration": "1h",
    "exclude_nodes": ["camera"],
    "include_only_nodes": null
  }
}
```

- `max_total_size`：录制总空间上限，达到后自动停止录制
- `max_duration`：最大录制时长
- `exclude_nodes`：排除高频/大数据量节点（如摄像头）
- `include_only_nodes`：只录制指定节点

---

## API 设计

### 录制控制

```
POST   /api/runs/:id/record/start     # 开始录制
POST   /api/runs/:id/record/stop      # 停止录制
GET    /api/runs/:id/record/status     # 录制状态
```

```json
// GET /api/runs/:id/record/status
{
  "recording": true,
  "started_at": "2026-04-01T14:00:00Z",
  "events_captured": 52144,
  "bytes_written": 134217728,
  "shards_count": 2,
  "elapsed": "5m23s"
}
```

### 回放控制

```
POST   /api/runs/:id/replay           # 开始回放（返回 replay session ID）
POST   /api/replay/:session/pause     # 暂停
POST   /api/replay/:session/resume    # 继续
POST   /api/replay/:session/seek      # 跳转到指定时间/序号
POST   /api/replay/:session/stop      # 停止回放
GET    /api/replay/:session/status     # 回放状态
```

### 录制数据访问

```
GET    /api/runs/:id/record/manifest   # 获取 manifest.json
GET    /api/runs/:id/record/events     # 分页查询事件（for 前端轻量回放）
         ?start_seq=0&limit=100
         &node_filter=node-a,node-b
```

```json
// GET /api/runs/:id/record/events?start_seq=0&limit=2
{
  "events": [
    {
      "seq": 1,
      "timestamp": "2026-04-01T14:00:00.123Z",
      "source": "node-a/output1",
      "target": "node-b/input1",
      "data_type": "utf8",
      "data_size": 256,
      "data_preview": "Hello, how can I help you?",
      "metadata": {"dm_type": "text/plain"}
    },
    {
      "seq": 2,
      "timestamp": "2026-04-01T14:00:01.456Z",
      "source": "node-b/output1",
      "target": "node-c/input1",
      "data_type": "large_binary",
      "data_size": 32768,
      "data_preview": "<binary: 32768 bytes, audio/pcm>",
      "metadata": {"dm_type": "audio/pcm", "sample_rate": 16000}
    }
  ],
  "has_more": true,
  "next_seq": 3
}
```

---

## 前端集成

### RuntimeGraphView 中的录制指示器

当 Run 正在录制时，在 RuntimeGraphView 的状态栏显示录制标识：

```
Status: Running  🔴 REC  52,144 events  128 MB
```

### 回放时间线（纯前端回放）

在 RuntimeGraphView 中嵌入时间线控件：

```
|◄  ◄◄  ▶  ►►  ►|   ──●──────────────────────  00:05:23 / 00:30:00   1.0x
```

- 播放/暂停/快进/快退/跳转
- 按时间线拖动 → 从对应 parquet 分片加载事件
- Edge 动画反映当前时间点的数据流动
- 点击 Node → NodeInspector 展示该时间点的事件内容

---

## 实现顺序建议

| Phase | 内容 | 依赖 |
|-------|------|------|
| R1 | dm-core: 录制数据模型 + parquet writer | pyarrow |
| R2 | dm-server: 录制 API (start/stop/status) | R1 |
| R3 | dm-server: 事件查询 API (GET /events) | R1 |
| R4 | Web: 录制状态指示器 + 事件列表浏览 | R2, R3 |
| R5 | Web: 纯前端时间线回放 (方式 B) | R3, R4 |
| R6 | 后端回放 (方式 A) — 可选，进阶 | R1, dora 能力评估 |

---

## 与架构原则的一致性

- ✅ **非节点** — 录制是平台能力，不污染节点
- ✅ **dm-core 节点无关** — 录制层不关心具体节点类型，只捕获 Arrow 事件
- ✅ **与存储族协调** — 通过 `DM_REPLAY` 环境变量，回放时存储族自动跳过
- ✅ **展示与持久化正交** — 录制只管持久化，回放的可视化由 RuntimeGraphView 负责
