## 目标

在 `crates/dm-core` 中，为所有公共 API 函数接入 `events::EventStore` 自动记录事件。
当任何操作被调用时，应自动向 `~/.dm/events.db` 写入一条 XES 兼容事件。

## 现有架构

- `crates/dm-core/src/events.rs`：已实现完整的 `EventStore`（SQLite 后端），包含：
  - `EventStore::open(home)` — 打开数据库
  - `EventStore::emit(event)` — 写入一条事件  
  - `EventBuilder::new(source, activity).case_id(...).message(...).attr(k,v).build()` — 构建事件
  - `EventSource` 枚举：`Core | Dataflow | Server | Frontend | Ci`
  - `EventLevel` 枚举：`Trace | Debug | Info | Warn | Error`

- `crates/dm-core/src/lib.rs`：暴露以下公共函数（都需要接入）：
  - `doctor(home)` → 记录 activity="doctor"
  - `versions(home)` → 记录 activity="versions.list"
  - `status(home, verbose)` → 记录 activity="status.check"
  - `up(home, verbose)` → 记录 activity="runtime.up"
  - `down(home, verbose)` → 记录 activity="runtime.down"
  - `uninstall(home, version)` → 记录 activity="version.uninstall", attr("version", version)
  - `use_version(home, version)` → 记录 activity="version.switch", attr("version", version)
  - `setup(home, verbose, progress_tx)` → 记录 activity="setup"
  - `passthrough(home, args, verbose)` → 记录 activity="passthrough", attr("args", args)

- `crates/dm-core/src/node.rs`：以下函数也需要接入：
  - `install_node(home, id)` → activity="node.install", attr("node_id", id)
  - `uninstall_node(home, id)` → activity="node.uninstall", attr("node_id", id)
  - `list_nodes(home)` → activity="node.list"
  - `node_status(home, id)` → activity="node.status", attr("node_id", id)

- `crates/dm-core/src/dataflow.rs`：
  - `transpile_graph(home, yaml_path)` → activity="dataflow.transpile", attr("path", yaml_path)

## 实施要求

1. **在每个函数的开头**，打开 EventStore 并 emit 一条事件（记录操作开始）。
2. **在函数返回时**（成功或失败），emit 第二条事件（记录操作结果）。
   - 成功：level=Info, message="OK" 或包含关键结果信息
   - 失败：level=Error, message=error.to_string()
3. **case_id** 使用格式 `"session_{uuid}"`，每次函数调用生成一个新的 UUID。
   需要添加 `uuid` 依赖到 `dm-core/Cargo.toml`：`uuid = { version = "1", features = ["v4"] }`
4. **EventStore 的打开不应阻断主逻辑**：如果 `EventStore::open()` 失败，
   应该静默跳过事件记录（用 `.ok()` 忽略），绝不能导致主函数返回错误。
5. 封装一个辅助宏或函数来减少样板代码，建议在 `events.rs` 底部添加：
   ```rust
   /// Try to emit an event, silently ignoring failures
   pub fn try_emit(home: &Path, event: Event) {
       if let Ok(store) = EventStore::open(home) {
           let _ = store.emit(&event);
       }
   }
   ```
6. **不要修改** `events.rs` 中已有的测试。
7. 所有注释用英文。

## 验证

完成后运行：
- `cargo build --workspace` — 必须 0 错误
- `cargo test -p dm-core` — 所有现有测试必须继续通过
