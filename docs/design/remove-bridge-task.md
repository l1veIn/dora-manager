# Remove Bridge — 任务拆解

## 概况

删除整个 bridge 子系统（dm-cli bridge 子进程 + dm-core bridge 注入逻辑 + dm-server bridge.sock 监听），将 6 个交互节点改为使用 dm SDK 直接与 dm-server 通信。

## 背景

bridge 是在 SDK 存在之前的过渡方案。它的工作：
1. **注入隐藏节点**：`__dm_bridge` 自动注入到 transpile 后的 dataflow YAML 中
2. **消息转发**：通过 Unix Domain Socket (bridge.sock) 把 dora Arrow 数据转发到 dm-server 的 Message Service
3. **输入回传**：从 dm-server 接收 input 事件，转换为 dora Arrow 数据发送给交互节点

现在 SDK 已经成熟，节点可以通过 HTTP POST /api/runs/{run_id}/messages 直接发送消息，通过 WebSocket 或轮询接收输入。不再需要 bridge。

## 任务拆解

### 任务 1: 删除 bridge Rust 代码

#### 1.1 dm-cli
- 删除 `crates/dm-cli/src/bridge.rs`（整个文件，355 行）
- 删除 `crates/dm-cli/src/main.rs` 中的 `mod bridge;`
- 删除 `main.rs` 中 `Commands::Bridge` 变体（第 108-113 行）
- 删除 `main.rs` 中 `Commands::Bridge { run_id } => bridge::bridge_serve(...)`（第 357 行）

#### 1.2 dm-core
- 删除 `crates/dm-core/src/dataflow/transpile/bridge.rs`（整个文件，239 行）
- 删除 `crates/dm-core/src/dataflow/transpile/passes.rs` 中的 `inject_dm_bridge` 函数（第 456-583 行，约 128 行）
- 删除 `passes.rs` 中的 `use super::bridge::*` 导入（第 3-7 行）
- 删除 `passes.rs` 中 `validate_reserved` 函数——它在 bridge 模式下是为了验证 __dm_bridge 不冲突，现在不再需要（第 106-113 行）
- 删除 `crates/dm-core/src/dataflow/transpile/mod.rs` 中 `mod bridge;`（第 11 行）
- 删除 `mod.rs` 中的 `passes::inject_dm_bridge(...)` 调用（第 69 行）
- 删除 `mod.rs` 的 pipeline 注释中第 6 步（第 9 行）

#### 1.3 dm-core/util.rs
- 删除 `DM_CLI_BIN_ENV_KEY` 常量（第 3 行）
- 删除 `resolve_dm_cli_exe()` 函数（第 19-28 行）
- 删除 `resolve_dm_cli_exe_from_path_or_sibling()` 函数（第 31-38 行）

#### 1.4 dm-server
- 删除 `crates/dm-server/src/handlers/bridge_socket.rs`（整个文件，174 行）
- 删除 `crates/dm-server/src/handlers/mod.rs` 中的 `pub(crate) mod bridge_socket;`（第 1 行）
- 删除 `crates/dm-server/src/main.rs` 中：
  - `configure_dm_cli_bridge_entrypoint();` 调用（第 86 行）
  - bridge.sock 启动代码块（第 288-301 行）
  - `configure_dm_cli_bridge_entrypoint` 函数（第 330-358 行）

### 任务 2: 更新测试

#### 2.1 dm-core 测试
- 更新 `crates/dm-core/src/tests/tests_dataflow.rs`：
  - 删除或重写 `transpile_graph_auto_injects_hidden_dm_bridge_for_v0_bindings` 测试
  - 删除或重写 `transpile_graph_skips_hidden_dm_bridge_without_supported_bindings` 测试
  - 删除或重写 `transpile_graph_injects_widget_bridge_input_mapping` 测试
  - 这些测试验证 bridge 注入行为，函数本身已删除，测试必须删除或改为验证 bridge 不再被注入

### 任务 3: SDK 化交互节点（Python 节点）

将 6 个节点改为使用 `dm` SDK 发送消息。

#### 3.1 dm-display — 展示节点（读取 dora 输入，通过 SDK 发到 Web UI）

当前逻辑（`nodes/dm-display/dm_display/main.py`，171 行）：
1. 从 dora 的 `path` 或 `data` 端口接收输入
2. 调用 `emit_bridge()` 通过 bridge channel 发送

SDK 化改造：
1. 保留 dora Node 事件循环（仍然需要接收 dora 输入）
2. 初始化 `msg = dm.Message()` 替代 bridge
3. 将 `emit_bridge()` 调用替换为 `msg.send(tag, payload)`
4. 删除 `DM_BRIDGE_OUTPUT_PORT` 环境变量依赖

注意：dm-display 有 input ports，不需要监听输入回传。它只是单向推送到 UI。

#### 3.2 dm-message — 消息节点（同 dm-display 模式）

当前逻辑（163 行）：
1. 从 dora 的 `message` 端口接收输入
2. 调用 `emit_bridge()` 通过 bridge channel 发送

SDK 化改造：
1. 用 `msg = dm.Message()` 替代 bridge
2. 将 `emit_bridge()` 替换为 `msg.send()`

#### 3.3 dm-slider — 交互输入节点（需要监听输入回传）

当前逻辑：
1. 监听 `bridge_input_port` 上的 dora 事件（来自 bridge 子进程）
2. 解析 payload 中的 `value` → 通过 `node.send_output()` 发送到 dora 数据流

SDK 化改造：
1. 仍然需要保持 `node.send_output()` 输出到 dora 数据流（与其他节点的连接不变）
2. 启动一个后台线程，用 `msg.subscribe()` 或 `msg.get()` 轮询 `tag="input"`、`to=node_id` 的消息
3. 收到后解析 `value` → 通过 `node.send_output()` 发送
4. 初始化时注册 widgets
5. 删除 `DM_BRIDGE_INPUT_PORT` 环境变量依赖

#### 3.4 dm-button — 同 dm-slider 模式

当前逻辑（80 行）：
1. 监听 bridge 输入事件
2. 解析 `value` → `node.send_output("click", ...)`

SDK 化改造：
1. 后台线程监听 `tag="input"` 指向自己的消息
2. 收到后解析 → `node.send_output("click", ...)`
3. 初始化时注册 widgets

#### 3.5 dm-text-input — 同 dm-slider 模式

当前逻辑（103 行）：
1. 监听 bridge 输入事件
2. 解析 `value` → `node.send_output("value", ...)`

SDK 化改造同 dm-slider。

#### 3.6 dm-input-switch — 同 dm-slider 模式

当前逻辑（76 行）：
1. 监听 bridge 输入事件
2. 解析 `value` → `node.send_output("value", ...)`

SDK 化改造同 dm-slider。

### 任务 4: 更新 dm.json （移除 capability binding）

交互节点的 capability binding 中 `widget_input` 的 `channel: "input"` binding 原先是通过 bridge 实现的。现在 SDK 接管后：

对于 **输出方向** 的 binding（widgets 注册等），SDK 通过 HTTP API 完成，不再需要 bridge binding。
对于 **输入方向** 的 binding（监听用户输入），SDK 通过 WebSocket/polling 完成。

因此：
- 保留 `channel: "register"` 的 binding（仍然表示"这个节点注册了控件"）
- 删除或标记 `channel: "input"` 的 binding（bridge 不再代理这个回路）
- 但注意：`dm.json` 中的 capability binding 目前只用于 transpile bridge 注入。如果 bridge 删了，binding 字段暂时只是元数据。**可以先保留不动**，因为它们是语义声明而非功能实现。后续再考虑重构 capability 系统。

### 任务 5: 更新 wiki 文档

需要更新以下 wiki 文件（中英文双语）：

#### 5.1 架构概览 — `wiki/zh/10-architecture-overview.md`（及英文版）
- 删除 bridge 描述和架构图中的 bridge 组件
- 更新为：节点通过 dm SDK（HTTP API）直连 dm-server 的 Message Service
- 更新交互回路图示

#### 5.2 交互系统 — `wiki/zh/22-interaction-system.md`（及英文版）
这是最重要的一份。需要：
- 删除所有关于 bridge、`__dm_bridge`、bridge.sock 的描述
- 重写交互回路：SDK 直接 HTTP POST 消息 → dm-server 存 SQLite → WebSocket 推前端
- 重写输入回路：前端 POST input → dm-server SQLite → 节点 SDK subscribe/poll 获取
- 更新数据流图

#### 5.3 响应式控件 — `wiki/zh/20-reactive-widgets.md`（及英文版）
- 删除 bridge 连接描述
- 更新 SDK 注册控件和接收输入的方式

#### 5.4 Transpiler — `wiki/zh/11-transpiler.md`（及英文版）
- 删除 bridge injection pass 描述
- 更新 pipeline 步骤（从 7 步变成 6 步）
- 删除 `__dm_bridge`、`DM_CAPABILITIES_JSON`、`DM_BRIDGE_INPUT_PORT`、`DM_BRIDGE_OUTPUT_PORT` 的说明

#### 5.5 Capability Binding — `wiki/zh/23-capability-binding.md`（及英文版）
- 删除 bridge 相关的 binding 传递逻辑说明
- 如果保留 binding 作为元数据，需要更新说明

#### 5.6 HTTP API — `wiki/zh/15-http-api.md`（及英文版）
- 删除 bridge.sock 的提及
- 删除 `/ws/node` 相关的 bridge 说明（但保留 WebSocket API）

#### 5.7 内置节点 — `wiki/zh/07-builtin-nodes.md`（及英文版）
- 更新 dm-slider/dm-button/dm-text-input/dm-input-switch 的描述
- 删除 bridge 依赖说明

#### 5.8 SDK v1 设计文档 — `docs/design/dm-sdk-v1.md`
- 更新"当前情况"中关于 bridge 的描述

#### 5.9 项目宪法 — `wiki/zh/27-project-constitution.md`（及英文版）
- 删除或更新 bridge 相关的条款

#### 5.10 其他文档
- `wiki/zh/04-node-concept.md`：删除 bridge env vars 说明
- `wiki/zh/08-port-schema.md`：删除或更新 bridge port 引用
- `wiki/zh/index.md`：更新概述
- `wiki/zh/05-dataflow-concept.md`：删除 bridge 提及
- `wiki/zh/16-config-system.md`：删除 bridge env 提及
- `wiki/zh/19-runtime-workspace.md`：删除 bridge 提及
- `wiki/zh/26-testing-strategy.md`：删除 bridge 相关测试说明
- `wiki/zh/24-media-streaming.md`：删除 bridge 提及
- `wiki/zh/03-dev-environment.md`：删除 bridge 相关
- `wiki/zh/06-run-lifecycle.md`：删除 bridge 提及
- `wiki/zh/12-node-management.md`：删除 bridge 提及
- `wiki/zh/17-sveltekit-structure.md`：删除 bridge 提及
- `docs/architecture-principles.md`：删除 bridge 引用
- `docs/dm-server-service-roadmap.md`：如果需要引用 bridge
- `docs/dm-custom-panel.md`：删除 bridge 引用
- `docs/design/dm-capability-binding-v0.md`：删除或更新 bridge 部分
- `docs/design/panel-ontology-memo.md`：删除 bridge 引用
- `docs/records/steering-cycles.md`：是否更新（历史记录）
- `docs/records/ux-test-rounds.md`：是否更新
- `docs/design/dm-sdk-v1.md`：更新"当前情况"部分

### 任务 6: 更新 demo dataflow YAML

- `demos/demo-interactive-widgets.yml` — 交互控件 demo，目前使用 dm-slider/dm-button/dm-text-input/dm-input-switch。SDK 化后这些节点无需 bridge，但 demo YAML 本身不需要修改（YAML 配置不变，节点内部实现变了）
- `demos/robotics-object-detection.yml` — 包含 dm-slider 和 dm-display，同上

**注意**：demo YAML 文件不需要修改。节点名和输入输出端口不变，变化只在节点内部 Python 代码。

### 任务 7: 验收标准

1. `cargo build` 成功，无警告
2. `cargo test` 全部通过
3. SDK 测试 `pytest sdk/python/tests/` 通过
4. 所有 wiki 文档中 bridge 相关的描述已删除或更新
5. 没有任何文件再引用 `bridge` `__dm_bridge` `bridge.sock` `DM_BRIDGE_INPUT_PORT` `DM_BRIDGE_OUTPUT_PORT` `DM_CAPABILITIES_JSON`（历史记录文档和 design 文档可保留作为记录）
