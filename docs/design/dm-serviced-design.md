# dm-serviced — Service Runtime Daemon

> 独立进程，持有 `SERVICE_RUNTIME`，暴露 Unix socket 让节点/CLI/server 调用 service。

## 动机

当前 `SERVICE_RUNTIME` 是 dm-core 里的 `static`，属于进程内存。谁调 `invoke_service()` 就在谁的进程里启一个，进程退出就没了。

这有两个问题：
1. CLI 退出后 worker 被 KillOnDrop 干掉，无法热复用
2. 节点（dora 子进程）无法直接调 `invoke_service()`，因为节点进程没有链接 dm-core

需要一个独立的长进程来持有 `SERVICE_RUNTIME`，让任何调用者都能通过它走热启动路径。

## 架构

```
dm-serviced (独立进程)
  │ 持有 SERVICE_RUNTIME
  │ 监听 ~/.dm/service.sock (Unix socket)
  │
  ├─ CLI: dm service invoke → 写 socket → serviced 执行 → 返回
  ├─ Server: POST /api/services/{id}/invoke → server 调 dm_core → serviced socket → 执行
  └─ 节点: 写 bridge socket → server → serviced socket → 执行

生命周期：
  up:   dm-core::service_runtime::up(home) → spawn serviced 进程
  down: dm-core::service_runtime::down(home) → kill serviced
  idle: serviced 自己监听到 5 分钟没人 invoke → 自动 exit
```

## 与 dora daemon 对称

| | dora daemon | dm-serviced |
|---|---|---|
| 进程 | `dora up` | `dm-serviced` |
| 管理函数 | `dm_core::dora::up()` / `down()` | `dm_core::service_runtime::up()` / `down()` |
| 生命周期 | dm(server/cli) 管理 | 自己 idle 退出 |
| 通信 | dora 内部 IPC | Unix socket JSON |
| 协议 | dora 自有协议 | 和 `invoke.rs` 一样的 stdin/stdout JSON |

## 新 crate
## 项目结构

```
dm-core (lib)                     ← 共享逻辑
  ├─ service invoke
  ├─ service_runtime::up() / down() / is_running()
  └─ SERVICE_RUNTIME (static)

dm-cli (bin)                      ← 用户交互
  └─ 链接 dm-core

dm-server (bin)                   ← Web / WS / Bridge
  └─ 链接 dm-core

dm-serviced (bin)                 ← 后台常驻 SERVICE_RUNTIME
  └─ 链接 dm-core，最小依赖
```

三个二进制，一个库。serviced 链接 dm-core 但不使用 jsonschema、rusqlite、reqwest 等重型依赖，release 体积约 4-5MB。

## 新 crate

`crates/dm-serviced/`

```toml
[package]
name = "dm-serviced"
version = "0.1.0"
edition = "2021"

[dependencies]
dm-core = { path = "../dm-core" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
serde_json = "1"
```
最小依赖，只有 dm-core + tokio + anyhow + serde_json。

入口 main.rs（约 100 行）：

```rust
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() {
    let home = dm_core::config::resolve_home(None).unwrap();
    let socket_path = home.join("service.sock");

    // 清理旧 socket
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path).unwrap();

    // 启动 reaper
    dm_core::service::start_service_runtime_reaper();

    // idle 追踪
    let last_invoke = Arc::new(AtomicU64::new(timestamp_secs()));

    // idle 自退出
    let idle_invoke = last_invoke.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let elapsed = timestamp_secs() - idle_invoke.load(Ordering::Relaxed);
            if elapsed > 300 {
                // 5 分钟无人 invoke，自退出
                std::process::exit(0);
            }
        }
    });

    // 主循环
    while let Ok((stream, _)) = listener.accept().await {
        let last_invoke = last_invoke.clone();
        let home = home.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 { break; }
                last_invoke.store(timestamp_secs(), Ordering::Relaxed);
                let result = handle_invoke(&home, line.trim()).await;
                // 写回结果（需要获取 write half）
                // ...
                line.clear();
            }
        });
    }
}

async fn handle_invoke(home: &Path, line: &str) -> Result<String> {
    // 解析请求：{"service_id": "message", "method": "send", "input": {...}, "context": {...}}
    // 调 dm_core::service::invoke_service()
    // 返回 JSON 响应
}

fn timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
```

serviced 只做路由，不做任何业务逻辑——业务在 Python service 里。

### 与 dora daemon 的对齐

| | dora daemon | dm-serviced |
|---|---|---|
| 角色 | 持有 dora 运行时，管理 dataflow 执行 | 持有 SERVICE_RUNTIME，管理 service worker |
| 二进制 | dora（子命令 daemon/coordinator/up/...） | dm-serviced |
| 通信 | TCP（端口 53290/6012，coordinator 控制面） | **Unix socket**（本地通信，无端口，文件权限保护） |
| 为什么不需要 coordinator | dataflow 有运行态需要查询（list/stop） | service 的状态在文件里（service.json/message.db），不需要运行态查询 |
| 管理函数 | `dm_core::dora::up()` / `down()` | `dm_core::service_runtime::up()` / `down()` |
| 自退出 | 外部 auto_down_if_idle | 自己 idle 5 分钟退出 |

## dm-core 新增 API

```rust
// dm-core/src/api/service_runtime.rs
pub async fn service_runtime_up(home: &Path) -> Result<RuntimeResult>;
pub async fn service_runtime_down(home: &Path) -> Result<RuntimeResult>;
pub fn is_service_runtime_running(home: &Path) -> bool;
```

up 的逻辑（复用 runtime.rs 里 `dora::up()` 的模式）：
1. 检查 `service.sock` 是否已存在（另一个 serviced 在跑）
2. 不存在则 spawn `dm-serviced` 子进程
3. 轮询等待 socket 就绪

down 的逻辑：
1. 发 SIGTERM 给 serviced 进程
2. 等待进程退出

## invoke_service 自动管理

```rust
// invoke_service 内部检测 serviced 状态
pub async fn invoke_service(home: &Path, id: &str, invocation: ServiceInvocation) -> Result<...> {
    if !is_service_runtime_running(home) {
        service_runtime_up(home).await?;
    }
    // 通过 socket 调 serviced 执行
    call_service_via_socket(home, id, invocation).await
}
```

CLI、server handler、bridge 都调同一个 `invoke_service()`，各自不需要写 up/down。

## 通信协议

Unix socket 上传输 JSON，每行一个完整对象：

```
→ {"service_id": "add", "method": "run", "input": {"x": 2, "y": 3}, "context": {}}
← {"service_id": "add", "method": "run", "output": {"result": 5}}
```

和现有 invoke 协议一致，只是传输方式从 stdin/stdout 换成了 socket。

## idle 自退出

serviced 启动时 spawn 一个后台任务，每 30 秒检查最后一次 invoke 的时间戳。
如果超过 5 分钟无人调用，`std::process::exit(0)`。

不需要外部进程来关它。CLI/Server 下次调 service 时会自动 up 一个新的。

## 不做的

1. **不做分布式** — 本地通信，Unix socket 足矣
2. **不做身份认证** — socket 文件权限天然保护
3. **不做高可用** — 单进程，挂了自动重新 up
4. **不做控制面** — service 没有运行态需要查询
