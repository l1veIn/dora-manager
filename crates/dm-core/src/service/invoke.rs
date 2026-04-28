use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;

use crate::events::{EventSource, OperationEvent};

use super::model::{Service, ServiceMethod};

const DEFAULT_SERVICE_ENTRY: &str = "service.py";
const SERVICE_SOCKET_NAME: &str = "service.sock";
const SERVICED_READY_TIMEOUT_MS: u64 = 5_000;
const SERVICED_READY_POLL_MS: u64 = 25;
const DM_SERVICED_BIN_ENV_KEY: &str = "DM_SERVICED_BIN";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInvocation {
    pub method: String,
    #[serde(default)]
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInvocationResult {
    pub service_id: String,
    pub method: String,
    pub output: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInvocationError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

impl ServiceInvocationError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        service_id: Option<String>,
        method: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            service_id,
            method,
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: serde_json::Value) -> Self {
        self.detail = Some(detail);
        self
    }
}

impl fmt::Display for ServiceInvocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ServiceInvocationError {}

pub async fn invoke_service(
    home: &Path,
    id: &str,
    invocation: ServiceInvocation,
) -> Result<ServiceInvocationResult> {
    let op = OperationEvent::new(home, EventSource::Core, "service.invoke")
        .attr("service_id", id)
        .attr("method", &invocation.method);
    op.emit_start();

    let result = async {
        let service = super::get_service(home, id)?.ok_or_else(|| {
            service_error(
                "service_not_found",
                format!("Service '{}' not found", id),
                id,
                None,
            )
        })?;
        let method = find_method(&service, &invocation.method)?;
        validate_json_schema(
            method.input_schema.as_ref(),
            &invocation.input,
            "input_schema_invalid",
            "input_validation_failed",
            &service.id,
            &invocation.method,
            "input",
        )?;

        let output = invoke_python_service(home, &service, &invocation).await?;

        validate_json_schema(
            method.output_schema.as_ref(),
            &output,
            "output_schema_invalid",
            "output_validation_failed",
            &service.id,
            &invocation.method,
            "output",
        )?;

        Ok(ServiceInvocationResult {
            service_id: service.id,
            method: invocation.method,
            output,
        })
    }
    .await;

    op.emit_result(&result);
    result
}

fn find_method<'a>(service: &'a Service, method: &str) -> Result<&'a ServiceMethod> {
    service
        .methods
        .iter()
        .find(|entry| entry.name == method)
        .ok_or_else(|| {
            service_error(
                "method_not_found",
                format!(
                    "Service '{}' does not declare method '{}'",
                    service.id, method
                ),
                &service.id,
                Some(method),
            )
            .into()
        })
}

async fn invoke_python_service(
    home: &Path,
    service: &Service,
    invocation: &ServiceInvocation,
) -> Result<serde_json::Value> {
    let socket_path = service_socket_path(home);
    ensure_serviced_running(home, &socket_path).await?;

    let stream = match UnixStream::connect(&socket_path).await {
        Ok(stream) => stream,
        Err(first_err) => {
            if socket_path.exists() {
                let _ = std::fs::remove_file(&socket_path);
                spawn_serviced(home)?;
                wait_for_serviced(&socket_path).await?;
                UnixStream::connect(&socket_path)
                    .await
                    .with_context(|| {
                        format!(
                            "failed to connect dm-serviced at {} after restart; first error: {first_err}",
                            socket_path.display()
                        )
                    })?
            } else {
                return Err(first_err).with_context(|| {
                    format!("failed to connect dm-serviced at {}", socket_path.display())
                });
            }
        }
    };

    let request = serde_json::json!({
        "service_id": service.id,
        "method": invocation.method,
        "input": invocation.input,
        "context": invocation.context,
    });
    let (read_half, mut write_half) = stream.into_split();
    let mut payload = serde_json::to_vec(&request)?;
    payload.push(b'\n');
    write_half.write_all(&payload).await?;
    write_half.flush().await?;

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        anyhow::bail!("dm-serviced closed the connection without a response");
    }

    let value: serde_json::Value =
        serde_json::from_str(line.trim()).context("dm-serviced returned invalid response JSON")?;
    if value.get("code").is_some() {
        let err: ServiceInvocationError = serde_json::from_value(value)
            .context("dm-serviced returned an invalid error response")?;
        return Err(err.into());
    }

    let result: ServiceInvocationResult = serde_json::from_value(value)
        .context("dm-serviced returned an invalid success response")?;
    Ok(result.output)
}

fn service_socket_path(home: &Path) -> PathBuf {
    home.join(SERVICE_SOCKET_NAME)
}

async fn ensure_serviced_running(home: &Path, socket_path: &Path) -> Result<()> {
    if socket_path.exists() {
        return Ok(());
    }
    spawn_serviced(home)?;
    wait_for_serviced(socket_path).await
}

fn spawn_serviced(home: &Path) -> Result<()> {
    let mut command = Command::new(resolve_serviced_bin());
    command
        .env("DM_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().context("failed to spawn dm-serviced")?;
    Ok(())
}

async fn wait_for_serviced(socket_path: &Path) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(SERVICED_READY_TIMEOUT_MS);
    loop {
        if socket_path.exists() && UnixStream::connect(socket_path).await.is_ok() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "dm-serviced socket did not become ready at {}",
                socket_path.display()
            );
        }
        tokio::time::sleep(Duration::from_millis(SERVICED_READY_POLL_MS)).await;
    }
}

fn resolve_serviced_bin() -> PathBuf {
    if let Some(path) = std::env::var_os(DM_SERVICED_BIN_ENV_KEY) {
        return PathBuf::from(path);
    }

    let exe_name = if cfg!(windows) {
        "dm-serviced.exe"
    } else {
        "dm-serviced"
    };

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let sibling = dir.join(exe_name);
            if sibling.exists() {
                return sibling;
            }
            if dir.file_name().is_some_and(|name| name == "deps") {
                if let Some(target_dir) = dir.parent() {
                    let target_sibling = target_dir.join(exe_name);
                    if target_sibling.exists() {
                        return target_sibling;
                    }
                }
            }
        }
    }

    PathBuf::from(exe_name)
}

#[allow(dead_code)]
pub(super) fn command_for_service(service: &Service) -> Result<Command> {
    if let Some(exec) = legacy_exec(service) {
        return Ok(command_for_exec(exec));
    }

    let entry = service_entry(service).ok_or_else(|| missing_entry_error(service))?;
    let mut command = Command::new(python_for_service(service));
    command.arg(entry);
    Ok(command)
}

#[allow(dead_code)]
pub(super) fn service_command_label(service: &Service) -> Result<String> {
    if let Some(exec) = legacy_exec(service) {
        return Ok(exec.to_string());
    }
    let entry = service_entry(service).ok_or_else(|| missing_entry_error(service))?;
    Ok(format!("{} {}", python_for_service(service), entry))
}

#[allow(dead_code)]
pub(super) fn legacy_exec(service: &Service) -> Option<&str> {
    service
        .runtime
        .exec
        .as_deref()
        .filter(|value| !value.trim().is_empty())
}

#[allow(dead_code)]
pub(super) fn service_entry(service: &Service) -> Option<String> {
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

#[allow(dead_code)]
pub(super) fn python_for_service(service: &Service) -> String {
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

#[allow(dead_code)]
pub(super) fn missing_entry_error(service: &Service) -> ServiceInvocationError {
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

#[allow(dead_code)]
pub(super) fn command_for_exec(exec: &str) -> Command {
    if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", exec]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-c", exec]);
        command
    }
}

fn validate_json_schema(
    schema: Option<&serde_json::Value>,
    value: &serde_json::Value,
    schema_error_code: &str,
    validation_error_code: &str,
    service_id: &str,
    method: &str,
    label: &str,
) -> Result<()> {
    let Some(schema) = schema else {
        return Ok(());
    };
    let validator = jsonschema::validator_for(schema).map_err(|err| {
        service_error(
            schema_error_code,
            format!(
                "Service '{}.{}' declares an invalid {} schema: {}",
                service_id, method, label, err
            ),
            service_id,
            Some(method),
        )
    })?;
    let errors = validator
        .iter_errors(value)
        .map(|err| {
            serde_json::json!({
                "path": err.instance_path().to_string(),
                "message": err.to_string(),
            })
        })
        .collect::<Vec<_>>();
    if errors.is_empty() {
        return Ok(());
    }

    Err(service_error(
        validation_error_code,
        format!(
            "Service '{}.{}' {} failed schema validation",
            service_id, method, label
        ),
        service_id,
        Some(method),
    )
    .with_detail(serde_json::json!({ "errors": errors }))
    .into())
}

pub(super) fn service_error(
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
