运行时服务是 Dora Manager 后端的核心引擎，负责将一份 YAML 数据流拓扑转化为一个受管理的 **运行实例（Run）**，并持续追踪其状态直至终止。该服务横跨 `dm-core` 的 `runs` 模块与 `dm-server` 的 HTTP/WebSocket 层，构成一个完整的"启动 → 监控 → 采集 → 终止"生命周期管理管线。本文将从架构概览出发，逐层深入启动编排、状态刷新与指标采集三大子系统，揭示各层之间的协作关系与设计权衡。

Sources: [mod.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/mod.rs#L1-L26), [service.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service.rs#L1-L45)

## 架构总览：三层分离与 Backend 抽象

运行时服务的代码组织遵循 **职责分层** 原则：`model` 层定义纯数据结构，`repo` 层封装文件系统 I/O，`service` 层组合业务逻辑。这种分层通过 `service.rs` 这个门面文件（facade）统一导出，外部调用者只需面对一组简洁的公共函数。

在所有分层之上，存在一个关键的设计抽象——**`RuntimeBackend` trait**。该 trait 将与 Dora CLI 二进制的所有交互（启动、停止、列举）封装为可替换的后端接口，使得核心业务逻辑可以在测试中被 mock，而在生产环境中使用 `DoraCliBackend`——一个通过 `tokio::process::Command` 调用 `dora` CLI 的实现。

```
┌─────────────────────────────────────────────────────────┐
│                    dm-server (HTTP/WS)                    │
│  ┌──────────┐  ┌───────────┐  ┌──────────────────────┐  │
│  │ runs.rs  │  │ run_ws.rs │  │ runtime.rs (up/down) │  │
│  └────┬─────┘  └─────┬─────┘  └──────────┬───────────┘  │
│       │              │                    │               │
├───────┼──────────────┼────────────────────┼───────────────┤
│       ▼              ▼                    ▼               │
│  ┌─────────────────────────────────────────────────────┐ │
│  │              dm-core::runs (service layer)           │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ │ │
│  │  │service_start │ │service_runtime│ │service_metrics│ │ │
│  │  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘ │ │
│  │         │                │                 │         │ │
│  │  ┌──────┴────────────────┴─────────────────┴──────┐  │ │
│  │  │           RuntimeBackend trait                  │  │ │
│  │  │         (start_detached / stop / list)          │  │ │
│  │  └──────────────────┬─────────────────────────────┘  │ │
│  │                     │                                │ │
│  │  ┌──────────────────┴─────────────────────────────┐  │ │
│  │  │           DoraCliBackend                        │  │ │
│  │  │  dora start --detach / dora stop / dora list    │  │ │
│  │  └────────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

Sources: [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L14-L37), [service.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service.rs#L14-L44)

## RunInstance 数据模型与文件系统布局

每个运行实例在磁盘上对应 `$DM_HOME/runs/<run_id>/` 目录，内部包含完整的状态快照、转译产物、日志与输出文件。`RunInstance` 是核心持久化模型，以 JSON 格式存储为 `run.json`。

**文件系统布局**：

| 路径 | 用途 |
|---|---|
| `run.json` | 运行实例元数据（状态、时间戳、dora_uuid 等） |
| `dataflow.yml` | 原始 YAML 快照 |
| `view.json` | 可选的画布视图状态（编辑器位置/缩放） |
| `dataflow.transpiled.yml` | 经过转译管线处理后的可执行 YAML |
| `logs/<node_id>.log` | 从 Dora 输出同步而来的节点日志 |
| `out/<dora_uuid>/log_<node>.txt` | Dora 运行时的原始输出目录 |

Sources: [repo.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/repo.rs#L1-L48)

**RunInstance 核心字段解析**：

| 字段 | 类型 | 说明 |
|---|---|---|
| `run_id` | `String` | UUID，全局唯一标识 |
| `dora_uuid` | `Option<String>` | Dora 运行时分配的 dataflow UUID |
| `status` | `RunStatus` | `Running` / `Succeeded` / `Stopped` / `Failed` |
| `termination_reason` | `Option<TerminationReason>` | 终止原因枚举（共 6 种） |
| `failure_node` / `failure_message` | `Option<String>` | 失败时定位到具体节点和错误摘要 |
| `outcome` | `RunOutcome` | 包含 `status`、`termination_reason`、`summary` 的人可读摘要 |
| `node_count_expected` / `node_count_observed` | `u32` | 预期节点数 vs 实际观测到的节点数 |
| `log_sync` | `RunLogSync` | 日志同步状态（`Pending` / `Synced`） |
| `source` | `RunSource` | 启动来源：`Cli` / `Server` / `Web` / `Unknown` |

Sources: [model.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/model.rs#L119-L174)

## 启动编排：从 YAML 到运行实例

启动一个 Run 是一个多阶段的编排过程。无论是通过 CLI 的 `dm run` 还是 HTTP API 的 `POST /api/runs/start`，最终都汇聚到 `start_run_from_yaml_with_source_and_strategy` 函数。以下流程图展示了完整的编排路径：

```
┌─────────────────┐
│  接收 YAML 输入  │
└────────┬────────┘
         ▼
┌─────────────────────────┐
│ Phase 1: 可执行性检查     │
│ inspect_yaml() → can_run?│
│ ├─ invalid_yaml → bail  │
│ └─ missing_nodes → bail │
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 2: 冲突检测         │
│ 同名 dataflow 是否已运行？ │
│ ├─ Fail → 报错退出       │
│ └─ StopAndRestart → 停止 │
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 3: 准备运行环境      │
│ ├─ 生成 run_id (UUID)    │
│ ├─ 创建目录布局           │
│ ├─ 保存原始 YAML 快照     │
│ ├─ 保存 view.json (可选) │
│ └─ 计算 dataflow_hash   │
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 4: 数据流转译       │
│ transpile_graph_for_run()│
│ → dataflow.transpiled.yml│
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 5: 构建并持久化      │
│ RunInstance 实例          │
│ → save_run(run.json)    │
└────────┬────────────────┘
         ▼
┌─────────────────────────┐
│ Phase 6: 调用 Dora 启动   │
│ backend.start_detached() │
│ ├─ Ok(Some(uuid)) → 成功│
│ ├─ Ok(None) → 标记失败   │
│ └─ Err → 标记失败        │
└─────────────────────────┘
```

Sources: [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L72-L220)

### Phase 1：可执行性检查

启动前首先调用 `inspect_yaml()` 对 YAML 进行静态分析。该函数解析 YAML 后逐一检查每个节点声明的路径，确认对应的 `dm.json` 文件是否存在于 `$DM_HOME/nodes/` 目录中。如果存在缺失节点（`missing_nodes`）或无效 YAML（`invalid_yaml`），则直接返回错误，避免将一个注定失败的数据流提交给 Dora 运行时。

Sources: [inspect.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/dataflow/inspect.rs#L19-L39), [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L81-L103)

### Phase 2：冲突检测与策略选择

`StartConflictStrategy` 枚举定义了两种冲突处理策略：**`Fail`**（默认，遇到同名运行中的 dataflow 直接报错）和 **`StopAndRestart`**（先停止已有运行再重新启动）。冲突检测通过 `find_active_run_by_name_with_backend()` 实现——该函数会先刷新所有运行状态，再在活跃 Run 中按 `dataflow_name` 匹配。

在 HTTP 层，`POST /api/runs/start` 通过 `force` 参数映射到策略选择。当未指定 `force` 或 `force=false` 时使用 `Fail` 策略；当 `force=true` 时使用 `StopAndRestart`。

Sources: [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L105-L118), [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L222-L228)

### Phase 3-5：环境准备与转译

生成 UUID 作为 `run_id`，创建完整的目录结构，将原始 YAML 保存为 `dataflow.yml` 快照。随后调用 `transpile_graph_for_run()` 执行多 Pass 转译管线（路径解析、端口校验、配置合并、运行时环境注入），产出 `dataflow.transpiled.yml`。最后构建 `RunInstance` 对象——此时 `status` 为 `Running`，`dora_uuid` 为 `None`，`outcome.summary` 为 "Running"——并持久化到磁盘。

Sources: [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L120-L174), [transpile/mod.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/dataflow/transpile/mod.rs#L36-L77)

### Phase 6：Dora 进程启动

`DoraCliBackend::start_detached()` 通过 `tokio::process::Command` 执行 `dora start <transpiled_path> --detach`。该命令以分离模式启动数据流，Dora CLI 返回后数据流在后台持续运行。启动成功的关键信号是输出中包含 `dataflow start triggered: <uuid>` 或 `dataflow started: <uuid>` 格式的行——`extract_dataflow_id()` 函数负责从中提取 `dora_uuid`。

如果启动成功但未返回 UUID，`RunInstance` 会被立即标记为 `Failed`（`termination_reason: StartFailed`）。如果启动过程本身抛出异常，同样标记为 `Failed` 并记录错误详情。

Sources: [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L39-L73), [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L176-L220)

### Server 层的额外保障

`dm-server` 的 `start_run` handler 在调用核心启动逻辑之前，还增加了两项保障：**媒体后端就绪检查**（如果数据流包含媒体节点但 MediaMTX 未就绪，则拒绝启动）和**运行时自动拉起**（调用 `ensure_runtime_up()` 确保 Dora daemon 正在运行）。`ensure_runtime_up()` 通过 `dora check` 探测运行时状态，若未运行则自动执行 `dora up`，最多等待 5 秒。

Sources: [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L195-L220), [api/runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/api/runtime.rs#L248-L256)

## 状态刷新：与 Dora 运行时的同步协议

`refresh_run_statuses()` 是状态管理的核心函数，负责将本地记录的 Run 状态与 Dora 运行时的真实状态进行同步。该函数在任何查询操作（`list_runs`、`get_run`、`list_active_runs`）之前被调用，确保调用者看到的状态是最新鲜的。

### 同步算法

```
refresh_run_statuses_with_backend():
  1. 调用 backend.list() 获取 Dora 运行时中所有活跃 dataflow
  2. 构建 runtime_map: {dora_uuid → RunStatus}
  3. 遍历本地所有 status.is_running() 的 Run:
     ├─ runtime_map 中找到 Running → 保持 Running
     ├─ runtime_map 中找到 Succeeded → 标记完成 + sync_run_outputs()
     ├─ runtime_map 中找到 Failed → 推断失败详情 + sync_run_outputs()
     ├─ runtime_map 中找到 Stopped → 标记 RuntimeStopped + sync_run_outputs()
     └─ runtime_map 中未找到 → 标记 RuntimeLost + sync_run_outputs()
```

**`RuntimeLost`** 是一个值得注意的状态——当本地记录显示某 Run 正在运行，但 Dora 运行时已不再报告该 dataflow 时，说明运行时发生了非预期的丢失（例如 Dora daemon 重启）。系统将其标记为 `Stopped` 并附带 `RuntimeLost` 原因，确保状态不会永远卡在 `Running`。

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L99-L214)

### 日志同步：sync_run_outputs()

当 Run 进入终态时，`sync_run_outputs()` 将 Dora 运行时输出的原始日志（`out/<dora_uuid>/log_<node>.txt`）复制到规范位置（`logs/<node_id>.log`），同时更新 `nodes_observed` 列表和 `log_sync` 状态。这个同步步骤确保了即使 Dora 运行时已经清理了原始输出目录，日志仍然可以在 `logs/` 目录中找到。

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L216-L263)

### 失败推断：infer_failure_details()

当 Dora 报告一个 dataflow 失败时，它只提供粗粒度的状态信息。`infer_failure_details()` 通过两步策略来丰富失败详情：首先尝试从 `dora stop` 的错误消息中解析 `"node &lt;name&gt; failed:"` 格式的信息；如果该信息为空，则遍历所有已观测节点的日志文件，用启发式规则（`AssertionError:`、`thread 'main' panicked at`、`ERROR`、Python Traceback 等）提取第一条错误摘要。

Sources: [state.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/state.rs#L34-L57), [state.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/state.rs#L116-L150)

### 终止状态转换：apply_terminal_state()

`apply_terminal_state()` 是所有终态转换的统一入口。它将 `TerminalStateUpdate` 中的信息应用到 `RunInstance`，同时调用 `build_outcome()` 生成人可读的摘要文本。该函数保证了 `stopped_at` 时间戳在首次转换时被设置（幂等），并将 `runtime_observed_at` 更新为观测时刻。

`build_outcome()` 的摘要生成逻辑如下表所示：

| 状态 | 条件 | 摘要文本 |
|---|---|---|
| `Running` | — | "Running" |
| `Succeeded` | — | "Succeeded" |
| `Stopped` | `StoppedByUser` | "Stopped by user" |
| `Stopped` | `RuntimeLost` | "Stopped after Dora runtime lost track of the dataflow" |
| `Stopped` | `RuntimeStopped` | "Stopped by Dora runtime" |
| `Failed` | 有 node + message | "Failed: \<node\> \<message\>" |
| `Failed` | 仅 node | "Failed: \<node\>" |
| `Failed` | 仅 message | "Failed: \<message\>" |

Sources: [state.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/state.rs#L59-L114)

## 指标采集：CPU、内存与节点级监控

指标采集系统通过 `DoraCliBackend` 调用 `dora list --format json` 和 `dora node list --format json --dataflow <uuid>` 两个 CLI 命令，分别获取数据流级别和节点级别的实时指标。

### 两级指标结构

**数据流级别**（`RunMetrics`）：

| 字段 | 类型 | 来源 |
|---|---|---|
| `cpu` | `Option<f64>` | `dora list` JSON 中的 `cpu` 字段（百分比） |
| `memory_mb` | `Option<f64>` | `dora list` JSON 中的 `memory` 字段（GB → MB 转换） |
| `nodes` | `Vec<NodeMetrics>` | `dora node list` 的逐节点详情 |

**节点级别**（`NodeMetrics`）：

| 字段 | 类型 | 来源示例 |
|---|---|---|
| `id` | `String` | `"dora-qwen"` |
| `status` | `String` | `"Running"` |
| `pid` | `Option<String>` | `"67842"` |
| `cpu` | `Option<String>` | `"23.7%"` |
| `memory` | `Option<String>` | `"1143 MB"` |

Sources: [service_metrics.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_metrics.rs#L1-L96), [model.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/model.rs#L205-L219)

### 采集模式

系统提供两种采集函数：

- **`get_run_metrics(home, run_id)`**：单 Run 指标采集。先加载 Run 确认其处于 `Running` 状态且持有 `dora_uuid`，再依次调用数据流级和节点级指标采集。
- **`collect_all_active_metrics(home)`**：批量采集所有活跃数据流的指标。先一次性获取所有数据流的聚合指标，再为每个 UUID 逐个采集节点级指标。该方法被 HTTP API `GET /api/runs/active?metrics=true` 使用。

两个函数都解析 **换行分隔 JSON**（NDJSON）格式——Dora CLI 以每行一个 JSON 对象的方式输出结果。解析器对格式错误具备容错能力：无法解析的行被静默跳过，缺失字段返回 `None`。

Sources: [service_metrics.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_metrics.rs#L14-L55), [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L64-L92)

## 停止与清理

### 停止流程

`stop_run()` 通过 `DoraCliBackend::stop()` 执行 `dora stop <dora_uuid>`，设置了 15 秒超时。停止过程包含一个**容错机制**：如果 `dora stop` 命令失败，系统会再次调用 `backend.list()` 检查该 dataflow 是否实际上已经不在运行。如果已不在，则仍然标记为 `Stopped`（`StoppedByUser`），避免误报 `Failed`。

在 HTTP 层，`POST /api/runs/:id/stop` 采用 **fire-and-forget** 模式——将停止操作 `tokio::spawn` 到后台任务中，HTTP 响应立即返回 `{"status": "stopping"}`，避免客户端因等待 `dora stop` 超时而阻塞。

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L16-L97), [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L258-L284)

### 清理与删除

`delete_run()` 删除 Run 目录及其中所有文件，同时删除关联的事件记录（`EventStore::delete_by_case_id()`）。`clean_runs(home, keep)` 保留最近的 `keep` 条记录，删除其余历史记录。

Sources: [service_admin.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_admin.rs#L1-L28)

## 实时推送：WebSocket 端点

`dm-server` 通过 `GET /api/runs/:id/ws` 提供 WebSocket 端点，实现运行时的实时日志流与指标推送。该端点建立后，在后台运行一个事件循环，通过 `tokio::select!` 同时处理四类事件：

| 事件源 | 间隔 | 推送内容 |
|---|---|---|
| 文件变更通知 (`notify` crate) | 实时 | `WsMessage::Logs`（新增日志行）和 `WsMessage::Io`（含 `[DM-IO]` 标记的交互行） |
| 指标轮询 | 1 秒 | `WsMessage::Metrics`（节点级指标）+ `WsMessage::Status`（Run 状态） |
| 心跳 | 10 秒 | `WsMessage::Ping` |
| 客户端消息 | — | 检测 `Close` 帧以断开连接 |

文件监听器使用 `notify::recommended_watcher` 监视日志目录。当日志目录从 `out/<dora_uuid>/`（Dora 实时输出）切换到 `logs/`（同步后的归档）时，监听器会自动切换到新目录。每次读取日志时维护 `log_offsets` 哈希表，只推送增量内容，避免重复传输。

Sources: [run_ws.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/run_ws.rs#L1-L149), [run_ws.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/run_ws.rs#L207-L228)

## 后台空闲监控

`dm-server` 启动时注册一个后台任务，每 30 秒执行 `auto_down_if_idle()`。该函数先调用 `refresh_run_statuses()` 更新所有 Run 状态（这可以检测到自然完成的数据流），然后检查是否还有活跃 Run。如果没有，则自动执行 `dora down` 关闭 Dora 运行时，释放系统资源。

Sources: [main.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/main.rs#L234-L241), [api/runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/api/runtime.rs#L260-L270)

## RuntimeBackend 设计解析

`RuntimeBackend` trait 定义了三个方法，覆盖了运行时交互的完整生命周期：

```rust
pub trait RuntimeBackend {
    fn start_detached<'a>(&'a self, home: &'a Path, transpiled_path: &'a Path) 
        -> BoxFutureResult<'a, (Option<String>, String)>;
    fn stop<'a>(&'a self, home: &'a Path, dora_uuid: &'a str) 
        -> BoxFutureResult<'a, ()>;
    fn list(&self, home: &Path) -> Result<Vec<RuntimeDataflow>>;
}
```

`DoraCliBackend` 作为默认实现，其方法与 Dora CLI 命令的对应关系为：

| Trait 方法 | CLI 命令 | 返回值 |
|---|---|---|
| `start_detached` | `dora start &lt;path&gt; --detach` | `(Option<uuid>, output_text)` |
| `stop` | `dora stop <uuid>` (15s 超时) | `()` |
| `list` | `dora list` | `Vec<RuntimeDataflow>` |

`start_detached` 返回 `BoxFutureResult`（即 `Pin<Box<dyn Future<Output = Result<T>> + Send>>`），这是因为启动操作需要异步等待子进程完成。而 `list` 是同步的（使用 `std::process::Command`），因为状态刷新路径需要快速返回。所有接受 `RuntimeBackend` 的内部函数都使用泛型约束 `B: RuntimeBackend`，使得单元测试可以注入 mock 后端。

Sources: [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L1-L37), [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/runtime.rs#L39-L119)

## HTTP API 路由总览

以下表格汇总了运行时服务暴露的所有 HTTP 端点：

| 方法 | 路径 | 核心函数 | 说明 |
|---|---|---|---|
| GET | `/api/runs` | `list_runs_filtered` | 分页查询，支持 status/search 过滤 |
| GET | `/api/runs/active` | `list_runs_filtered` + `collect_all_active_metrics` | 活跃 Run 列表（可选附带指标） |
| GET | `/api/runs/:id` | `get_run` + `get_run_metrics` | Run 详情（可选附带指标） |
| GET | `/api/runs/:id/metrics` | `get_run_metrics` | 单 Run 指标 |
| POST | `/api/runs/start` | `start_run_from_yaml_with_source_and_strategy` | 启动新 Run |
| POST | `/api/runs/:id/stop` | `stop_run` | 停止 Run（后台异步） |
| POST | `/api/runs/delete` | `delete_run` | 批量删除 Run |
| GET | `/api/runs/:id/dataflow` | `read_run_dataflow` | 原始 YAML 快照 |
| GET | `/api/runs/:id/transpiled` | `read_run_transpiled` | 转译后的 YAML |
| GET | `/api/runs/:id/view` | `read_run_view` | 画布视图 JSON |
| GET | `/api/runs/:id/logs/:node` | `read_run_log` | 完整节点日志 |
| GET | `/api/runs/:id/logs/:node/tail` | `read_run_log_chunk` | 增量日志（offset 参数） |
| GET | `/api/runs/:id/ws` | WebSocket handler | 实时日志 + 指标推送 |
| POST | `/api/up` | `ensure_runtime_up` | 启动 Dora 运行时 |
| POST | `/api/down` | `down` | 关闭 Dora 运行时 |

Sources: [runs.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runs.rs#L1-L333), [runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/handlers/runtime.rs#L69-L84), [main.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-server/src/main.rs#L174-L213)

## 设计亮点与权衡

**防御性状态管理**：`refresh_run_statuses()` 在 `backend.list()` 失败时静默返回（`Err(_) => return Ok(())`），而不是传播错误。这是一个刻意的设计选择——当 Dora daemon 不可达时，系统不会将所有活跃 Run 误判为失败，而是保留最后已知状态，等待下次刷新机会。

**分离模式启动**：使用 `--detach` 标志启动数据流意味着 `dm-server` 进程不会成为数据流的父进程。即使服务器重启，已启动的数据流仍然在 Dora daemon 的管理下继续运行——服务器重启后通过 `refresh_run_statuses()` 重新发现并接管这些"孤儿"Run。

**容错停止**：`dora stop` 在某些边缘情况下（节点已自行退出、超时等）可能返回错误，但 dataflow 实际上已经停止。二次确认机制避免了将这类场景误判为 `Failed`。

**指标采集的性能考量**：`collect_all_active_metrics()` 对每个活跃 dataflow 都调用 `dora node list`，这意味着 N 个活跃 dataflow 会触发 N+1 次 CLI 调用（1 次 `dora list` + N 次 `dora node list`）。当前设计以进程调用的开销换取了实现的简洁性——对于典型的 1-3 个活跃 dataflow 场景，这是可接受的。

Sources: [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L108-L116), [service_start.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_start.rs#L176-L180), [service_runtime.rs](https://github.com/l1veIn/dora-manager/blob/master/crates/dm-core/src/runs/service_runtime.rs#L50-L77)

---

**延伸阅读**：了解运行实例的完整生命周期概念与状态机，参见 [运行实例（Run）：生命周期、状态与指标追踪](06-run-lifecycle)。了解数据流转译管线的多 Pass 处理细节，参见 [数据流转译器：多 Pass 管线与四层配置合并](08-transpiler)。了解 HTTP API 的完整路由定义与 Swagger 文档，参见 [HTTP API 路由全览与 Swagger 文档](12-http-api)。了解运行时产生的事件如何被记录和导出，参见 [事件系统：可观测性模型与 XES 兼容存储](11-event-system)。