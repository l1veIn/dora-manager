use std::collections::HashMap;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::{Mutex, OwnedMutexGuard};

use dm_core::service::{Service, ServiceInvocationError};

const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;
const REAPER_INTERVAL_SECS: u64 = 30;
const DEFAULT_SERVICE_ENTRY: &str = "service.py";

pub(super) static SERVICE_RUNTIME: ServiceRuntime = ServiceRuntime::new();

type WorkerRef = Arc<Mutex<ServiceWorker>>;

pub fn start_service_runtime_reaper() {
    SERVICE_RUNTIME.start_reaper();
}

pub(super) struct ServiceRuntime {
    managers: OnceLock<Mutex<HashMap<String, Arc<Mutex<ServiceProcessManager>>>>>,
}

struct ServiceProcessManager {
    service_id: String,
    service_path: String,
    workers: Vec<WorkerRef>,
    max_workers: usize,
    idle_timeout: Duration,
}

struct ServiceWorker {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: Option<ChildStderr>,
    last_used: Instant,
}

impl ServiceRuntime {
    const fn new() -> Self {
        Self {
            managers: OnceLock::new(),
        }
    }

    fn managers(&self) -> &Mutex<HashMap<String, Arc<Mutex<ServiceProcessManager>>>> {
        self.managers.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub(super) async fn invoke(
        &self,
        service: &Service,
        method: &str,
        input: Value,
        context: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let manager = self.manager_for(service).await;
        let request = serde_json::json!({
            "method": method,
            "input": input,
            "context": context,
        });

        let mut worker = {
            let mut manager = manager.lock().await;
            manager.acquire(service).await?
        };

        let result = worker.call(service, method, request.clone(), timeout).await;
        if result.is_ok() {
            return result;
        }

        let can_try_legacy = service.runtime.max_workers.is_none()
            && service.runtime.idle_timeout_secs.is_none()
            && is_timeout_error(&result);
        if worker.is_alive() {
            return result;
        }

        drop(worker);
        {
            let mut manager = manager.lock().await;
            manager.retain_live_workers().await;
        }

        if can_try_legacy {
            return invoke_one_shot(service, method, request, timeout).await;
        }

        let mut retry_worker = {
            let mut manager = manager.lock().await;
            manager.acquire(service).await?
        };
        retry_worker.call(service, method, request, timeout).await
    }

    fn start_reaper(&'static self) {
        let managers = self.managers();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(REAPER_INTERVAL_SECS));
            loop {
                interval.tick().await;
                let entries = {
                    let map = managers.lock().await;
                    map.values().cloned().collect::<Vec<_>>()
                };

                for manager in entries {
                    let mut manager = manager.lock().await;
                    manager.reap_idle_workers().await;
                }

                let mut map = managers.lock().await;
                map.retain(|_, manager| {
                    if let Ok(manager) = manager.try_lock() {
                        !manager.workers.is_empty()
                    } else {
                        true
                    }
                });
            }
        });
    }

    async fn manager_for(&self, service: &Service) -> Arc<Mutex<ServiceProcessManager>> {
        let key = service_key(service);
        let managers = self.managers();
        let mut map = managers.lock().await;
        map.entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(ServiceProcessManager::new(service))))
            .clone()
    }
}

impl ServiceProcessManager {
    fn new(service: &Service) -> Self {
        Self {
            service_id: service.id.clone(),
            service_path: service.path.display().to_string(),
            workers: Vec::new(),
            max_workers: service.runtime.max_workers.unwrap_or(0),
            idle_timeout: Duration::from_secs(
                service
                    .runtime
                    .idle_timeout_secs
                    .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS),
            ),
        }
    }

    async fn acquire(&mut self, service: &Service) -> Result<OwnedMutexGuard<ServiceWorker>> {
        self.refresh_config(service);
        self.retain_live_workers().await;

        for worker in &self.workers {
            if let Ok(guard) = worker.clone().try_lock_owned() {
                return Ok(guard);
            }
        }

        if self.max_workers == 0 || self.workers.len() < self.max_workers {
            let worker = Arc::new(Mutex::new(ServiceWorker::spawn(service).await?));
            let guard = worker.clone().lock_owned().await;
            self.workers.push(worker);
            return Ok(guard);
        }

        anyhow::bail!(
            "Service '{}' has no available workers (max_workers={}, path={})",
            self.service_id,
            self.max_workers,
            self.service_path
        );
    }

    fn refresh_config(&mut self, service: &Service) {
        self.max_workers = service.runtime.max_workers.unwrap_or(0);
        self.idle_timeout = Duration::from_secs(
            service
                .runtime
                .idle_timeout_secs
                .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS),
        );
    }

    async fn retain_live_workers(&mut self) {
        let mut retained = Vec::with_capacity(self.workers.len());
        for worker in self.workers.drain(..) {
            match worker.clone().try_lock_owned() {
                Ok(mut guard) => {
                    if guard.is_alive() {
                        drop(guard);
                        retained.push(worker);
                    } else {
                        guard.shutdown().await;
                    }
                }
                Err(_) => retained.push(worker),
            }
        }
        self.workers = retained;
    }

    async fn reap_idle_workers(&mut self) {
        let mut retained = Vec::with_capacity(self.workers.len());
        for worker in self.workers.drain(..) {
            match worker.clone().try_lock_owned() {
                Ok(mut guard) => {
                    if !guard.is_alive() || guard.last_used.elapsed() >= self.idle_timeout {
                        guard.shutdown().await;
                    } else {
                        drop(guard);
                        retained.push(worker);
                    }
                }
                Err(_) => retained.push(worker),
            }
        }
        self.workers = retained;
    }
}

impl ServiceWorker {
    async fn spawn(service: &Service) -> Result<Self> {
        let command_label = service_command_label(service)?;
        let mut command = command_for_service(service)?;
        let mut child = command
            .kill_on_drop(true)
            .current_dir(&service.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to start service command '{}'", command_label))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for service '{}'", service.id))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdout for service '{}'", service.id))?;
        let stderr = child.stderr.take();

        Ok(Self {
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
            stderr,
            last_used: Instant::now(),
        })
    }

    async fn call(
        &mut self,
        service: &Service,
        method: &str,
        request: Value,
        timeout: Duration,
    ) -> Result<Value> {
        let result = tokio::time::timeout(timeout, self.call_inner(service, method, request)).await;
        match result {
            Ok(result) => result,
            Err(_) => {
                self.shutdown().await;
                Err(service_error(
                    "timeout",
                    format!(
                        "Service '{}.{}' timed out after {}ms",
                        service.id,
                        method,
                        timeout.as_millis()
                    ),
                    &service.id,
                    Some(method),
                )
                .into())
            }
        }
    }

    async fn call_inner(
        &mut self,
        service: &Service,
        method: &str,
        request: Value,
    ) -> Result<Value> {
        let mut payload = serde_json::to_vec(&request)
            .with_context(|| format!("Failed to write request to service '{}'", service.id))?;
        payload.push(b'\n');
        self.stdin
            .write_all(&payload)
            .await
            .with_context(|| format!("Failed to write request to service '{}'", service.id))?;
        self.stdin
            .flush()
            .await
            .with_context(|| format!("Failed to finalize request for service '{}'", service.id))?;

        let mut line = String::new();
        let bytes = self
            .stdout
            .read_line(&mut line)
            .await
            .with_context(|| format!("Failed to read output from service '{}'", service.id))?;
        self.last_used = Instant::now();

        if bytes == 0 {
            let (status, stderr) = self.finished_status_and_stderr().await;
            if status.as_ref().is_some_and(ExitStatus::success) {
                return Err(invalid_output_error(service, method, "").into());
            }
            return Err(command_failed_error(service, method, status, stderr).into());
        }

        serde_json::from_str(line.trim())
            .map_err(|_| invalid_output_error(service, method, line.trim()).into())
    }

    fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    async fn shutdown(&mut self) {
        if self.is_alive() {
            let _ = self.child.kill().await;
        }
    }

    async fn finished_status_and_stderr(&mut self) -> (Option<ExitStatus>, String) {
        let status = self.child.wait().await.ok();
        let stderr = self.read_stderr().await;
        (status, stderr)
    }

    async fn read_stderr(&mut self) -> String {
        let Some(mut stderr) = self.stderr.take() else {
            return String::new();
        };
        let mut buffer = String::new();
        let _ = stderr.read_to_string(&mut buffer).await;
        buffer.trim().to_string()
    }
}

fn service_key(service: &Service) -> String {
    format!("{}:{}", service.id, service.path.display())
}

fn invalid_output_error(service: &Service, method: &str, stdout: &str) -> ServiceInvocationError {
    service_error(
        "invalid_output_json",
        format!("Service '{}.{}' returned invalid JSON", service.id, method),
        &service.id,
        Some(method),
    )
    .with_detail(serde_json::json!({
        "stdout": stdout,
    }))
}

fn command_failed_error(
    service: &Service,
    method: &str,
    status: Option<ExitStatus>,
    stderr: String,
) -> ServiceInvocationError {
    let status_label = status
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown status".to_string());
    let mut err = service_error(
        "command_failed",
        format!(
            "Service '{}.{}' command failed with status {}{}",
            service.id,
            method,
            status_label,
            if stderr.is_empty() {
                String::new()
            } else {
                format!(": {}", stderr)
            }
        ),
        &service.id,
        Some(method),
    );
    if !stderr.is_empty() {
        err = err.with_detail(serde_json::json!({
            "stderr": stderr,
            "status": status_label,
        }));
    }
    err
}

async fn invoke_one_shot(
    service: &Service,
    method: &str,
    request: Value,
    timeout: Duration,
) -> Result<Value> {
    let command_label = service_command_label(service)?;
    let mut command = command_for_service(service)?;
    let mut child = command
        .kill_on_drop(true)
        .current_dir(&service.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to start service command '{}'", command_label))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for service '{}'", service.id))?;
        let payload = serde_json::to_vec(&request)
            .with_context(|| format!("Failed to write request to service '{}'", service.id))?;
        stdin
            .write_all(&payload)
            .await
            .with_context(|| format!("Failed to write request to service '{}'", service.id))?;
        stdin
            .write_all(b"\n")
            .await
            .with_context(|| format!("Failed to finalize request for service '{}'", service.id))?;
    }

    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(output) => output
            .with_context(|| format!("Failed to read output from service '{}'", service.id))?,
        Err(_) => {
            return Err(service_error(
                "timeout",
                format!(
                    "Service '{}.{}' timed out after {}ms",
                    service.id,
                    method,
                    timeout.as_millis()
                ),
                &service.id,
                Some(method),
            )
            .into());
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(command_failed_error(service, method, Some(output.status), stderr).into());
    }

    serde_json::from_slice(&output.stdout).map_err(|_| {
        invalid_output_error(
            service,
            method,
            String::from_utf8_lossy(&output.stdout).trim(),
        )
        .into()
    })
}

fn is_timeout_error(result: &Result<Value>) -> bool {
    result
        .as_ref()
        .err()
        .and_then(|err| err.downcast_ref::<ServiceInvocationError>())
        .is_some_and(|err| err.code == "timeout")
}

fn command_for_service(service: &Service) -> Result<tokio::process::Command> {
    if let Some(exec) = legacy_exec(service) {
        return Ok(command_for_exec(exec));
    }

    let entry = service_entry(service).ok_or_else(|| missing_entry_error(service))?;
    let mut command = tokio::process::Command::new(python_for_service(service));
    command.arg(entry);
    Ok(command)
}

fn service_command_label(service: &Service) -> Result<String> {
    if let Some(exec) = legacy_exec(service) {
        return Ok(exec.to_string());
    }
    let entry = service_entry(service).ok_or_else(|| missing_entry_error(service))?;
    Ok(format!("{} {}", python_for_service(service), entry))
}

fn legacy_exec(service: &Service) -> Option<&str> {
    service
        .runtime
        .exec
        .as_deref()
        .filter(|value| !value.trim().is_empty())
}

fn service_entry(service: &Service) -> Option<String> {
    if let Some(entry) = service
        .entry
        .as_deref()
        .or(service.files.entry.as_deref())
        .filter(|value| !value.trim().is_empty())
    {
        return Some(entry.to_string());
    }

    service
        .path
        .join(DEFAULT_SERVICE_ENTRY)
        .exists()
        .then(|| DEFAULT_SERVICE_ENTRY.to_string())
}

fn python_for_service(service: &Service) -> String {
    let venv_python = if cfg!(windows) {
        service
            .path
            .join(".venv")
            .join("Scripts")
            .join("python.exe")
    } else {
        service.path.join(".venv").join("bin").join("python")
    };
    if venv_python.exists() {
        return venv_python.to_string_lossy().to_string();
    }

    "python3".to_string()
}

fn missing_entry_error(service: &Service) -> ServiceInvocationError {
    service_error(
        "runtime_not_configured",
        format!(
            "Service '{}' has no entry script. Add '{}' or set entry in service.json.",
            service.id, DEFAULT_SERVICE_ENTRY
        ),
        &service.id,
        None,
    )
}

fn command_for_exec(exec: &str) -> tokio::process::Command {
    if cfg!(windows) {
        let mut command = tokio::process::Command::new("cmd");
        command.args(["/C", exec]);
        command
    } else {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", exec]);
        command
    }
}

fn service_error(
    code: impl Into<String>,
    message: impl Into<String>,
    service_id: &str,
    method: Option<&str>,
) -> ServiceInvocationError {
    ServiceInvocationError::new(
        code,
        message,
        Some(service_id.to_string()),
        method.map(ToString::to_string),
    )
}
