# dm-core 节点无关化重构：Panel 清零

> Status: **Plan finalized, pending execution**

## 目标

新分支，全量删除 panel 和 test-harness 相关代码。DM 未发布，无兼容包袱，不 deprecated，直接清零。

## 设计方向（已确认，本次不实现）

- Panel → 后续拆为上行/下行两个节点，走 server IPC 中继（类 Tauri 模式）
- PanelStore → 后续由 dm-store 节点族取代
- dm-test-harness → 后续重新设计，不再依赖 core 特殊处理

---

## Phase 1: dm-core 清零

### Transpiler

#### [MODIFY] [model.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/model.rs)
- 删除 `DmNode::Panel` 变体

#### [MODIFY] [passes.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/transpile/passes.rs)
- 删除 `RESERVED_NODE_IDS`、`is_reserved_node_id()`
- 删除 `inject_panel()` pass
- 删除 `inject_test_harness()` pass
- `parse()` 不再特殊归类保留 ID
- `transpile()` 流水线移除这两个 pass

### Runs Model

#### [MODIFY] [model.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/model.rs)
- 删除：`has_panel`（RunInstance、RunSummary）、`panel_node_ids`（RunTranspileMetadata）、`has_panel`（RunListFilter）

#### [MODIFY] [graph.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/graph.rs)
- 删除 `build_transpile_metadata()` 中所有 panel 检测逻辑

#### [MODIFY] [service_start.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/service_start.rs)
- 删除 widgets 写入 + PanelStore 预播种整段
- 删除 `has_panel` 计算

#### [MODIFY] [service_query.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/service_query.rs)
- 删除 `has_panel` 映射和过滤

#### [MODIFY] [inspect.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/dataflow/inspect.rs)
- 删除硬编码跳过 `dm-panel` / `dm-test-harness`

### Panel 子模块

#### [DELETE] [panel/](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/panel/)
- 整个目录删除（mod.rs、model.rs、store.rs）
- `runs/mod.rs` 删除 `pub mod panel;`

#### [MODIFY] [repo.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/runs/repo.rs)
- 删除 `run_panel_dir()`
- `runs/mod.rs` 移除其 re-export

### 其他

#### [MODIFY] [types.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/types.rs)
- 删除 `has_panel` 字段

#### [MODIFY] [api/runtime.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/api/runtime.rs)
- 删除 `has_panel` 映射

### Tests

#### [MODIFY] [tests_types.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-core/src/tests/tests_types.rs)
- 删除所有 `has_panel` 字段赋值

#### [MODIFY] tests/dataflows/*.yml
- 所有测试 dataflow 中的 `dm-panel` 节点、相关连线、widgets 声明：直接删除清理

---

## Phase 2: dm-server 清零

#### [DELETE] [handlers/panel.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/panel.rs)
- 整文件删除

#### [DELETE] [handlers/panel_ws.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/panel_ws.rs)
- 整文件删除

#### [MODIFY] [handlers/mod.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/mod.rs)
- 删除 `mod panel;`、`mod panel_ws;` 及其 re-export

#### [MODIFY] [main.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-server/src/main.rs)
- 删除所有 panel 相关路由（`/api/runs/{id}/panel/*`）

#### [MODIFY] [handlers/runs.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/runs.rs)
- 删除 `has_panel` 查询参数

#### [MODIFY] [handlers/run_ws.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-server/src/handlers/run_ws.rs)
- 删除 `n.id != "dm-panel"` 过滤

#### [MODIFY] [tests.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-server/src/tests.rs)
- 删除所有 panel 相关测试（`setup_panel_run`、`query_panel_*`、`send_panel_*` 等）
- 删除剩余测试中的 `has_panel` 字段

---

## Phase 3: dm-cli 清零

#### [DELETE] [builtin/panel.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/builtin/panel.rs)
- 整文件删除

#### [DELETE] [builtin/test_harness.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/builtin/test_harness.rs)
- 整文件删除

#### [MODIFY] [builtin/mod.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/builtin/mod.rs)
- 删除 `pub mod panel;`、`pub mod test_harness;`（如果 mod 为空则删除整个 builtin/）

#### [MODIFY] [main.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/main.rs)
- 删除 `PanelCommands` 枚举
- 删除 `Commands::Panel` 及其 match 分支
- 删除 test harness 相关的子命令及 match 分支

#### [MODIFY] [display.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/display.rs)
- 表格中删除 "Panel" 列

#### [MODIFY] [cmd/test.rs](file:///Users/yangchen/Desktop/dora-manager/crates/dm-cli/src/cmd/test.rs)
- 删除对 test_harness 的引用（如果整个 test 命令依赖 harness 则整体删除）

---

## Phase 4: Web 前端清零

#### [DELETE] [PanelPane.svelte](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/PanelPane.svelte)
#### [DELETE] [PanelControls.svelte](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/PanelControls.svelte)
#### [DELETE] [PanelMessage.svelte](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/PanelMessage.svelte)
#### [DELETE] [panel/](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/panel/) 整个目录
- 所有 display widgets（VideoWidget、AudioWidget、ImageWidget、JsonWidget、FileWidget）
- 所有 control widgets（ControlSelect、ControlCheckbox、ControlRadio、ControlButton）

#### [MODIFY] [+page.svelte](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/+page.svelte)
- 删除 PanelPane 引用和渲染

#### [MODIFY] [RunSummaryCard.svelte](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/RunSummaryCard.svelte)
- 删除 "Panel Present" badge

#### [MODIFY] [RunHeader.svelte](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/RunHeader.svelte)
- 删除 `has_panel` 条件

#### [MODIFY] [+page.svelte](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/+page.svelte) (列表页)
- 删除 `has_panel` 过滤器

#### [MODIFY] [types.ts](file:///Users/yangchen/Desktop/dora-manager/web/src/routes/runs/%5Bid%5D/types.ts)
- 删除 panel 相关类型定义

---

## Verification

```bash
cargo check --workspace
cargo test --workspace
cd web && npm run check
```

- 不含 panel 的 dataflow 正常 start / stop
- Web Runs 列表和详情页无报错
- `dm list` 正常显示

---

## 后续路线图（本次不执行）

```
本次: Panel 清零  →  v2: dm-store 节点  →  v3: Panel IPC 重生
```
