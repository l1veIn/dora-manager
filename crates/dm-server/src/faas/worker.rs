/// Worker pool for managing Python subprocess workers.
///
/// Each function gets its own `WorkerPool`. Workers communicate with
/// the Python process via STDIN/STDOUT JSON lines.
///
/// Protocol:
///   → one JSON line per invocation
///   ← one JSON line per result

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::info;

use crate::faas::types::Function;

/// Holds the pools for all known functions.
pub struct Pools {
    pools: Mutex<HashMap<String, Arc<WorkerPool>>>,
    idle_timeout: Duration,
}

impl Pools {
    pub fn new(idle_timeout_secs: u64) -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            idle_timeout: Duration::from_secs(idle_timeout_secs),
        }
    }

    /// Get or create a pool for the given function.
    pub async fn for_function(&self, func: &Function) -> Arc<WorkerPool> {
        let mut pools = self.pools.lock().await;
        pools
            .entry(func.id.clone())
            .or_insert_with(|| Arc::new(WorkerPool::new(func, self.idle_timeout)))
            .clone()
    }

    /// Remove pools that have no live workers (idle cleanup).
    pub async fn reap(&self) {
        let mut pools = self.pools.lock().await;
        pools.retain(|id, pool| {
            if pool.is_empty_now() {
                info!("removing empty pool: {}", id);
                false
            } else {
                true
            }
        });
    }

    /// Total number of workers across all pools.
    pub async fn total_workers(&self) -> usize {
        let pools = self.pools.lock().await;
        let mut total = 0;
        for pool in pools.values() {
            total += pool.len().await;
        }
        total
    }

    /// Number of function pools.
    pub async fn pool_count(&self) -> usize {
        self.pools.lock().await.len()
    }
}

// ── Worker Pool ───────────────────────────────────────────────────

pub struct WorkerPool {
    service_id: String,
    service_path: String,
    entry: String,
    max_workers: usize,
    idle_timeout: Duration,
    workers: Mutex<Vec<WorkerRef>>,
}

type WorkerRef = Arc<Mutex<Worker>>;

impl WorkerPool {
    fn new(func: &Function, idle_timeout: Duration) -> Self {
        Self {
            service_id: func.id.clone(),
            service_path: func.path.display().to_string(),
            entry: func.entry.clone(),
            max_workers: func.runtime.max_workers,
            idle_timeout,
            workers: Mutex::new(Vec::new()),
        }
    }

    /// Acquire a worker — reuse an idle one or spawn a new one.
    pub async fn acquire(&self) -> Result<OwnedMutexGuard<Worker>> {
        let mut workers = self.workers.lock().await;
        // Try reuse idle
        for w in workers.iter() {
            if let Ok(mut guard) = w.clone().try_lock_owned() {
                if guard.is_alive() {
                    info!("reusing idle worker for {}", self.service_id);
                    return Ok(guard);
                }
            }
        }
        // Spawn new
        if workers.len() < self.max_workers {
            let worker = Worker::spawn(&self.service_path, &self.entry).await?;
            let worker = Arc::new(Mutex::new(worker));
            let guard = worker.clone().lock_owned().await;
            workers.push(worker);
            info!("spawned worker {}/{} for {}", workers.len(), self.max_workers, self.service_id);
            return Ok(guard);
        }
        anyhow::bail!("all {} workers for '{}' are busy", self.max_workers, self.service_id);
    }

    /// Remove dead workers.
    pub async fn reap_idle(&self) {
        let mut workers = self.workers.lock().await;
        let mut retained = Vec::with_capacity(workers.len());
        for w in workers.drain(..) {
            match w.clone().try_lock_owned() {
                Ok(mut guard) => {
                    let idle = guard.last_used.elapsed() >= self.idle_timeout;
                    if !guard.is_alive() || idle {
                        guard.shutdown().await;
                        info!("reaped worker for {} (alive={}, idle={})", self.service_id, guard.is_alive(), idle);
                    } else {
                        drop(guard);
                        retained.push(w);
                    }
                }
                Err(_) => retained.push(w),
            }
        }
        *workers = retained;
    }

    pub async fn len(&self) -> usize {
        self.workers.lock().await.len()
    }

    fn is_empty_now(&self) -> bool {
        self.workers.try_lock().map(|w| w.is_empty()).unwrap_or(false)
    }
}

// ── Worker (single Python subprocess) ──────────────────────────────

pub struct Worker {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: Option<ChildStderr>,
    pub last_used: Instant,
}

impl Worker {
    async fn spawn(service_path: &str, entry: &str) -> Result<Self> {
        let entry_path = std::path::Path::new(service_path).join(entry);
        if !entry_path.exists() {
            // Fallback to "service.py"
            let fallback = std::path::Path::new(service_path).join("service.py");
            if fallback.exists() {
                return Self::spawn_python(service_path, "service.py").await;
            }
            anyhow::bail!("entry script '{}' not found in {}", entry, service_path);
        }
        Self::spawn_python(service_path, entry).await
    }

    async fn spawn_python(service_path: &str, entry: &str) -> Result<Self> {
        let mut child = tokio::process::Command::new("python3")
            .arg(entry)
            .current_dir(service_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("failed to spawn python3 {entry} in {service_path}"))?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("no stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
        let stderr = child.stderr.take();

        Ok(Self {
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
            stderr,
            last_used: Instant::now(),
        })
    }

    /// Send a JSON request and read a JSON response.
    pub async fn call(&mut self, request: &Value) -> Result<Value> {
        let mut payload = serde_json::to_vec(request)?;
        payload.push(b'\n');
        self.stdin.write_all(&payload).await?;
        self.stdin.flush().await?;
        self.last_used = Instant::now();

        let mut line = String::new();
        let n = self.stdout.read_line(&mut line).await?;
        if n == 0 {
            let stderr = self.read_stderr().await;
            anyhow::bail!("worker closed stdout; stderr: {}", stderr);
        }
        let value: Value = serde_json::from_str(line.trim())?;
        Ok(value)
    }

    fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    async fn shutdown(&mut self) {
        if self.is_alive() {
            let _ = self.child.kill().await;
            let _ = self.child.wait().await;
        }
    }

    async fn read_stderr(&mut self) -> String {
        let Some(mut stderr) = self.stderr.take() else {
            return String::new();
        };
        let mut buf = String::new();
        let _ = tokio::io::AsyncReadExt::read_to_string(&mut stderr, &mut buf).await;
        buf.trim().to_string()
    }
}
