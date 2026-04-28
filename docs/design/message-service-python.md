# Message Service — 独立 Python 实现

> 将 message 业务逻辑从 dm-server Rust 代码中剥离，独立为 Python Service。

## 动机

当前 message 实现在 dm-server 的 Rust 代码中，混合了三层职责：
- 数据模型 + SQLite 操作（`services/message.rs`）
- Service invoke 编排（`services/invocation.rs`）
- HTTP/WS handler（`handlers/messages.rs`）

独立为 Python Service 的好处：
- message 可独立开发、调试、更新，不依赖 dm-server 编译
- 验证 Service 架构原则："所有功能的入口都是 Service invoke"
- 降低 dm-server 的 Rust 代码量，提升可维护性

## 架构变化

```
当前：
前端 / node bridge → dm-server → MessageService (Rust/SQLite)

改造后：
  send/list/snapshots:
    前端 / node bridge → dm-server (代理) → invoke("message", ...) → Python Service → SQLite

  WebSocket + artifact 文件服务:
    继续留在 dm-server 中（不适合通过 stdin/stdout 传输）
```

## Service 接口

方法、输入、输出匹配现有 `services/message/service.json`，**不做任何协议变更**。

### send

```json
Input:  {"from": "node_a", "tag": "text", "payload": {"content": "hello"}, "timestamp": 1234567890}
Output: {"seq": 42}
```

- `from`、`tag`、`payload` 必填
- `timestamp` 可选，不传则服务端生成
- 自动 upsert `message_snapshots` 表

### list

```json
Input:  {"run_id": "run_xxx", "after_seq": 10, "tag": ["text"], "limit": 100}
Output: {"messages": [...], "next_seq": 42}
```

- 所有字段可选
- `from`/`tag` 支持 `["*"]` 表示全部
- `after_seq`/`before_seq` 做 seq 范围过滤
- `limit` 默认 200

### snapshots

```json
Input:  {"run_id": "run_xxx"}
Output: {"snapshots": [{"node_id": "...", "tag": "...", "payload": {...}, "seq": 42, "updated_at": ...}]}
```

返回此 run 中所有 node/tag 的最新 snapshot。

## Python 实现

### 文件结构

```
services/message/
├── service.json          ← 现有 manifest（不动）
├── service.py            ← 新增：入口，stdin/stdout 循环
├── message_db.py         ← 新增：SQLite 封装
├── normalize.py          ← 新增：payload 标准化逻辑
└── README.md             ← 现有文档（不动）
```

### service.py — 入口

```python
#!/usr/bin/env python3
import sys, json

from message_db import MessageDB
from normalize import normalize_payload

db = MessageDB()  # init lazily on first call

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    method = request.get("method")
    input_data = request.get("input", {})
    context = request.get("context", {})

    run_id = context.get("run_id")
    if not run_id:
        result = {"error": {"code": "context_required", "message": "message service requires context.run_id"}}
    else:
        try:
            result = dispatch(method, run_id, input_data)
        except Exception as e:
            result = {"error": {"code": "internal", "message": str(e)}}

    sys.stdout.write(json.dumps(result) + "\n")
    sys.stdout.flush()
```

### message_db.py — SQLite

使用统一数据库 `~/.dm/services/message/message.db`，按 `run_id` 列逻辑隔离。

```sql
-- schema
CREATE TABLE IF NOT EXISTS messages (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id      TEXT NOT NULL,
    node_id     TEXT NOT NULL,
    tag         TEXT NOT NULL,
    payload     TEXT NOT NULL,  -- JSON text
    timestamp   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_messages_run ON messages(run_id, seq);
CREATE INDEX IF NOT EXISTS idx_messages_run_tag ON messages(run_id, node_id, tag, seq);

CREATE TABLE IF NOT EXISTS message_snapshots (
    run_id      TEXT NOT NULL,
    node_id     TEXT NOT NULL,
    tag         TEXT NOT NULL,
    payload     TEXT NOT NULL,
    seq         INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (run_id, node_id, tag)
);
```

**关键变化：现在的代码按 run 建独立 db 文件（`~/.dm/runs/<run_id>/interaction.db`），新版用统一 db + run_id 列。**

好处：
- 跨 run 查询可行
- 数据库连接只开一次，不需要每次 invoke 都 open
- 清理旧 run 时只需要 DELETE 而不是删文件

#### push (send)

```python
def push(self, run_id, node_id, tag, payload, timestamp=None):
    ts = timestamp or int(time.time())
    payload_json = json.dumps(payload, ensure_ascii=False)
    with self.conn:
        cur = self.conn.execute(
            "INSERT INTO messages (run_id, node_id, tag, payload, timestamp) VALUES (?, ?, ?, ?, ?)",
            (run_id, node_id, tag, payload_json, ts)
        )
        seq = cur.lastrowid
        self.conn.execute(
            """INSERT INTO message_snapshots (run_id, node_id, tag, payload, seq, updated_at)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(run_id, node_id, tag) DO UPDATE SET
                   payload = excluded.payload,
                   seq = excluded.seq,
                   updated_at = excluded.updated_at""",
            (run_id, node_id, tag, payload_json, seq, ts)
        )
    return seq
```

#### list

逐个字段过滤，与现有 Rust 逻辑一致。用 Python 实现，SQL 查询加 WHERE 条件。

```python
def list(self, run_id, after_seq=None, before_seq=None, from_filter=None,
         tag=None, limit=200, desc=False):
    conditions = ["run_id = ?"]
    params = [run_id]
    # ... 按条件拼接
```

#### snapshots

```python
def snapshots(self, run_id):
    rows = self.conn.execute(
        "SELECT node_id, tag, payload, seq, updated_at FROM message_snapshots WHERE run_id = ? ORDER BY node_id, tag",
        (run_id,)
    ).fetchall()
    return [{"node_id": r[0], "tag": r[1], "payload": json.loads(r[2]), "seq": r[3], "updated_at": r[4]} for r in rows]
```

### normalize.py — payload 标准化

从现有 Rust `normalize_payload` 和 `normalize_stream_payload` 翻译过来：

```python
def normalize_payload(tag, payload):
    if tag == "input":
        return payload
    if tag == "stream":
        return normalize_stream_payload(payload)
    # file path normalization
    if "file" in payload:
        payload = dict(payload)
        payload["file"] = normalize_relative_path(payload["file"])
    return payload
```

场景：来自 dora runtime 节点的 message，文件路径是相对 `run_out_dir` 的。当前 Rust 代码通过 `normalize_relative_path()` 做路径安全校验（防止路径穿越），Python 实现需要同样逻辑。

### 不需要做的（留在 dm-server 中）

| 功能 | 理由 |
|------|------|
| **WebSocket 推送** | 通过 broadcast channel 推给前端，不是 Service stdin/stdout 的职责 |
| **artifact 文件服务** | 文件下载是 HTTP 静态服务，不是消息服务 |
| **Unix socket bridge** | 桥接 dora runtime，消息中转逻辑不变 |
| **stream_descriptor 解析** | 依赖 MediaRuntime 状态，需要 dm-server 的 state |

这些功能在 dm-server 中保持不动，只是把数据读写的底层调用从 `MessageService::open()` 换成 `invoke("message", ...)`。

## 迁移计划

### Phase 1：Python Service 实现（这个文档的范围）

1. 创建 `services/message/service.py` + `message_db.py` + `normalize.py`
2. service.json 里加 `runtime: {"kind": "daemon", "max_workers": 1}`（不需要多 worker，message 的 SQLite 是单线程安全的）
3. `services/message/service.json` 加 `"entry": "service.py"`
4. 跑通 `dm service invoke message send '{"from":"test","tag":"text","payload":{"hello":"world"}}'`（需要提供 context.run_id）

### Phase 2：dm-server 桥接（后续）

- `services/invocation.rs`：删除 `invoke_message_service`（不再特殊处理 message）
- `handlers/messages.rs`：handler 内部调 `dm_core::service::invoke_service(home, "message", ...)` 代替直接调 `MessageService::open()`
- `handlers/bridge_socket.rs`：同上
- 删除 `services/message.rs`（Rust 版 MessageService）
- 删除 Rust 的 `interaction.db` 创建逻辑（由 Python 完成）

### Phase 3：清理

- 删除旧的 per-run `interaction.db` 文件
- Python service 标注 `"builtin": true`，随 dm-server 启动自动安装

## 不会做的事

1. **不改 service.json** — 方法名、schema 完全不变
2. **不引入外部依赖** — 只用 Python 标准库 + sqlite3
3. **不改变 WebSocket/HTTP 端点** — 前端对消息服务的调用方式不变
4. **不做跨 run 查询的 UI** — 数据层支持了，但前端不需要知道
