# Service Runtime 实现方案

> 基于现有 `invoke.rs` 的冷热启动 + 进程池方案。

## 现状

当前的 `invoke_service` 每次调用都会 spawn 一个 Python 子进程：

```
每次 invoke：
  1. 解析 service.json
  2. 校验 method 和 input schema
  3. spawn python service.py
  4. stdin 写入 JSON
  5. 读 stdout，等 timeout
  6. 校验 output schema
  7. 返回结果
  → 进程退出
```

这等价于 "冷启动 + 用完就扔"。对于调试阶段完全够用，但对于频繁调用的 Service（如 message、config），每次 spawn 的开销会累积。

## 目标

对现有 `invoke_service` 做最小侵入的改造，增加一层进程管理。

改造范围：**新增一个 `runtime.rs` 模块，修改 `invoke.rs` 40 行**。不改变 `model.rs`、`local.rs`、`install.rs`。

```
invoke.rs 的 invoke_python_service:
  当前：spawn → 写 stdin → 等 stdout → 返回
  改造后：检查进程池 → 复用/新建进程 → 写 stdin → 等 stdout → 返回
```

所有进程管理的逻辑封装在 `runtime.rs` 中，`invoke.rs` 只做一次委托调用。

## 模块设计

```
crates/dm-core/src/service/
├── mod.rs
├── model.rs           # 数据模型（不动）
├── local.rs           # CRUD（不动）
├── paths.rs           # 路径（不动）
├── invoke.rs          # 调用入口（改动 40 行）
├── install.rs         # 安装（不动）
├── import.rs          # 导入（不动）
├── tests.rs           # 测试（不动）
└── runtime.rs         # 新增：进程管理器
```

## runtime.rs 核心逻辑

```rust
// ============= 核心数据结构 =============

struct ServiceRuntime {
    /// service_id → 进程管理器
    runtimes: Mutex<HashMap<String, ServiceProcessManager>>,
}

struct ServiceProcessManager {
    service_id: String,
    /// 当前活的进程
    workers: Vec<ServiceWorker>,
    /// 最大 worker 数（0 = 不限制，默认 0）
    max_workers: usize,
    /// 空闲多久后回收（默认 5 分钟）
    idle_timeout: Duration,
    /// 最近一次调用时间
    last_used: Instant,
}

struct ServiceWorker {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    /// 进程启动时间
    started_at: Instant,
    /// 进程是否正在处理请求
    busy: bool,
}
```

```rust
// ============= 对外接口 =============

impl ServiceRuntime {
    /// 获取或创建一个进程管理器
    fn get_or_init(home: &Path, service: &Service) -> Result<&mut ServiceProcessManager>;

    /// 定时回收空闲进程（后台任务，每秒跑一次）
    async fn reap_loop(home: &Path);
}

impl ServiceProcessManager {
    /// 选择一个可用的 worker，如果没有则新建
    async fn acquire(&mut self) -> Result<&mut ServiceWorker>;

    /// 通过 worker 的 stdin 发送请求，读 stdout 返回
    async fn invoke(&mut self, method: &str, input: Value, ctx: Option<Value>) -> Result<Value>;

    /// 回收空闲超时的 worker
    fn reap(&mut self);

    /// 停止所有 worker
    async fn shutdown(&mut self);
}
```

```rust
// ============= Worker 通信协议 =============

impl ServiceWorker {
    /// 启动一个新进程
    async fn spawn(service: &Service) -> Result<Self>;

    /// 发送请求并等待响应
    async fn call(&mut self, request: Value, timeout: Duration) -> Result<Value>;
}
```

## 通信协议

Service Worker 和 Core 通过 stdin/stdout 交换 JSON：

```text
► Core → stdin：
  {"method": "get_level", "input": {}, "context": {"run_id": "abc"}}

◄ Worker → stdout：
  {"level": 85}

出错时：
◄ Worker → stdout：
  {"error": {"code": "internal", "message": "something broke"}}
```

单行 JSON（每行一个完整的 JSON 对象）。进程启动后不会自动退出，等待下一条指令。

Protocol 和当前的 `invoke_python_service` 完全一致，不改变协议——只是把"进程只活一次调用"改成"进程持续等待输入"。

## 改动量

### invoke.rs 改动（约 40 行）

现有 `invoke_python_service` 函数（~100 行）替换为：

```rust
async fn invoke_python_service(
    home: &Path,
    service: &Service,
    invocation: &ServiceInvocation,
) -> Result<Value> {
    let request = serde_json::json!({
        "method": invocation.method,
        "input": invocation.input,
        "context": invocation.context,
    });

    let timeout = Duration::from_millis(
        service.timeout_ms
            .or(service.runtime.timeout_ms)
            .unwrap_or(DEFAULT_SERVICE_TIMEOUT_MS),
    );

    // 委托给进程管理器
    let runtime = SERVICE_RUNTIME.get_or_init(home, service)?;
    runtime.invoke(&invocation.method, invocation.input.clone(), invocation.context.clone()).await
}
```

### runtime.rs 新增（约 200 行）

完整的代码骨架见下文。

## Runtime 完整代码

```rust
// crates/dm-core/src/service/runtime.rs

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;

use super::model::Service;

// ── 客户端管理 ──

struct ServiceProcessManager {
    service_id: String,
    workers: Vec<ServiceWorker>,
    max_workers: usize,
    idle_timeout: Duration,
    last_used: Instant,
}

struct ServiceWorker {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    started_at: Instant,
    busy: bool,
}

impl ServiceWorker {
    async fn spawn(service: &Service) -> Result<Self> {
        let entry = service_entry(service)
            .ok_or_else(|| anyhow::anyhow!("Service '{}' has no entry", service.id))?;

        let python = python_for_service(service);
        let mut child = Command::new(&python)
            .arg(&entry)
            .current_dir(&service.path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("Failed to spawn service '{}'", service.id))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for service '{}'", service.id))?;

        Ok(Self {
            child,
            stdin: BufWriter::new(stdin),
            started_at: Instant::now(),
            busy: false,
        })
    }

    async fn call(&mut self, request: Value, timeout: Duration) -> Result<Value> {
        // 发送
        let mut payload = serde_json::to_vec(&request)?;
        payload.push(b'\n');
        self.stdin.write_all(&payload).await?;
        self.stdin.flush().await?;
        self.busy = true;

        // 读取 stdout 一行
        let mut reader = BufReader::new(
            self.child.stdout.take()
                .ok_or_else(|| anyhow::anyhow!("Failed to read stdout"))?
        );
        let mut line = String::new();
        let result = tokio::time::timeout(timeout, reader.read_line(&mut line)).await;

        self.busy = false;

        match result {
            Ok(Ok(0)) => {
                anyhow::bail!("Service worker exited unexpectedly");
            }
            Ok(Ok(_)) => {
                let value: Value = serde_json::from_str(&line)
                    .with_context(|| format!("Invalid JSON from service: {}", line.trim()))?;
                Ok(value)
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                anyhow::bail!("Service call timed out after {}ms", timeout.as_millis());
            }
        }
    }

    fn is_alive(&mut self) -> bool {
        // 用 try_wait 检查进程
        self.child.try_wait().ok().flatten().is_none()
    }

    fn idle_for(&self) -> Duration {
        self.started_at.elapsed()
    }
}

impl ServiceProcessManager {
    fn new(service_id: &str) -> Self {
        Self {
            service_id: service_id.to_string(),
            workers: Vec::new(),
            max_workers: 0,
            idle_timeout: Duration::from_secs(300), // 5 分钟
            last_used: Instant::now(),
        }
    }

    /// 获取一个可用的 worker，没有则新建
    async fn acquire(&mut self, service: &Service) -> Result<&mut ServiceWorker> {
        // 清理已退出的进程
        self.workers.retain(|w| {
            // 无法检查 alive 而不 borrow，退出时通过 reap 清理
            true
        });

        // 找空闲的活 worker
        if let Some(idx) = self.workers.iter().position(|w| !w.busy && w.is_alive()) {
            return Ok(&mut self.workers[idx]);
        }

        // 如果未达上限，新建
        if self.max_workers == 0 || self.workers.len() < self.max_workers {
            let worker = ServiceWorker::spawn(service).await?;
            self.workers.push(worker);
            return Ok(self.workers.last_mut().unwrap());
        }

        // 所有 worker 都忙，返回最后一个让其等待
        anyhow::bail!(
            "Service '{}' all {} workers are busy",
            self.service_id,
            self.workers.len()
        );
    }

    async fn invoke(
        &mut self,
        service: &Service,
        method: &str,
        input: Value,
        context: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        self.last_used = Instant::now();

        let request = serde_json::json!({
            "method": method,
            "input": input,
            "context": context,
        });

        let worker = self.acquire(service).await?;
        let result = worker.call(request, timeout).await;

        // 如果 worker 挂了，重试一次（冷启动补偿）
        if result.is_err() && !worker.is_alive() {
            self.workers.retain(|w| w.is_alive());
            let new_worker = ServiceWorker::spawn(service).await?;
            self.workers.push(new_worker);
            return new_worker.call(request, timeout).await;
        }

        result
    }

    /// 回收空闲超时的 worker
    fn reap(&mut self) {
        if self.workers.is_empty() {
            return;
        }

        let timeout = self.idle_timeout;
        self.workers.retain(|w| {
            if w.busy {
                return true;
            }
            // 保留至少一个 worker
            if self.workers.len() <= 1 {
                return true;
            }
            w.idle_for() < timeout
        });
    }

    fn shutdown(&mut self) {
        for worker in &mut self.workers {
            let _ = worker.child.kill().await;
        }
        self.workers.clear();
    }
}

// ── 全局运行时 ──

pub(crate) struct ServiceRuntime {
    map: Mutex<HashMap<String, ServiceProcessManager>>,
}

impl ServiceRuntime {
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    pub async fn invoke(
        &self,
        home: &Path,
        service: &Service,
        method: &str,
        input: Value,
        context: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let mut map = self.map.lock().unwrap();
        let mgr = map
            .entry(service.id.clone())
            .or_insert_with(|| ServiceProcessManager::new(&service.id));
        mgr.invoke(service, method, input, context, timeout).await
    }

    /// 后台回收线程
    pub fn start_reaper(self: &Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let mut map = this.map.lock().unwrap();
                map.retain(|_id, mgr| {
                    mgr.reap();
                    // 如果没有任何 worker 了，移除 entry（释放内存）
                    !mgr.workers.is_empty()
                });
            }
        });
    }
}

// ── 辅助函数（复用 invoke.rs 的） ──

fn service_entry(service: &Service) -> Option<String> {
    // 复用 invoke.rs 中的相同逻辑
    // 导入时可以直接引用 invoke.rs 的函数
}

fn python_for_service(service: &Service) -> String {
    // 复用 invoke.rs 中的相同逻辑
}
```

## 改动清单

| 文件 | 改动 | 行数 |
|---|---|---|
| `invoke.rs` | `invoke_python_service` 委托给 `SERVICE_RUNTIME` | ~40 行 |
| `runtime.rs` | 新增：进程管理、worker 复用、回收 | ~200 行 |
| `mod.rs` | 引入 `runtime` 模块 | +2 行 |
| `dm-server` | 启动时触发 `reaper` 后台任务 | +3 行 |

**总计新增代码：约 200 行。** 没有任何依赖新增。

## 冷热启动行为

```
第一次 invoke（无进程）：
  冷启动 → spawn python → 执行 → 返回 → 进程保持存活

第二次 invoke（10 秒后）：
  热启动 → 找到空闲 worker → stdin 写 → 读 stdout → 返回

第五次 invoke（10 分钟后）：
  进程因空闲超时被 reap → 冷启动 → spawn 新进程 → 返回

高并发场景（同时 3 个 invoke）：
  第一个 worker 忙 → 第二个 worker 忙 → 第三个 worker 忙
  → 新建第四个 worker（如果没达上限）
  → 或返回"all workers busy"
```

## 配置项

在 `service.json` 中扩展：

```json
{
  "id": "yolo",
  "methods": [...],
  "runtime": {
    "kind": "daemon",
    "max_workers": 2,
    "idle_timeout_secs": 300
  }
}
```

- `runtime.kind` 保持现有枚举值不变，新增的进程管理不改变此字段含义
- `runtime.max_workers`：可选，0 = 不限制（默认）
- `runtime.idle_timeout_secs`：可选，默认 300（5 分钟）

## 不会做的事

1. **不改 `model.rs` 的数据结构**——`ServiceRuntime` 和 `ServiceRuntimeKind` 保持现有定义。进程管理是运行时行为，不是 service.json 的驱动字段（`max_workers`/`idle_timeout` 是可选的附加配置）
2. **不改 invoke 协议**——stdin/stdout JSON 单行格式不变
3. **不改 install 流程**——安装时只创建 venv 和装依赖，不启动进程
4. **不加新的依赖**——只用 tokio 和 std
5. **不引入 HTTP server**——进程管理走管道，不走端口
