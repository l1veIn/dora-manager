use std::fmt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::events::{EventSource, OperationEvent};

use super::model::{Service, ServiceMethod};

const DEFAULT_SERVICE_ENTRY: &str = "service.py";
const DEFAULT_SERVICE_TIMEOUT_MS: u64 = 10_000;

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

#[derive(Debug, Clone, Serialize)]
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
    fn new(
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

    fn with_detail(mut self, detail: serde_json::Value) -> Self {
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

        let output = invoke_python_service(&service, &invocation).await?;

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
    service: &Service,
    invocation: &ServiceInvocation,
) -> Result<serde_json::Value> {
    let request = serde_json::json!({
        "method": invocation.method,
        "input": invocation.input,
        "context": invocation.context,
    });
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

    let timeout = Duration::from_millis(
        service
            .timeout_ms
            .or(service.runtime.timeout_ms)
            .unwrap_or(DEFAULT_SERVICE_TIMEOUT_MS),
    );
    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(output) => output
            .with_context(|| format!("Failed to read output from service '{}'", service.id))?,
        Err(_) => {
            return Err(service_error(
                "timeout",
                format!(
                    "Service '{}.{}' timed out after {}ms",
                    service.id,
                    invocation.method,
                    timeout.as_millis()
                ),
                &service.id,
                Some(&invocation.method),
            )
            .into());
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let mut err = service_error(
            "command_failed",
            format!(
                "Service '{}.{}' command failed with status {}{}",
                service.id,
                invocation.method,
                output.status,
                if stderr.is_empty() {
                    String::new()
                } else {
                    format!(": {}", stderr)
                }
            ),
            &service.id,
            Some(&invocation.method),
        );
        if !stderr.is_empty() {
            err = err.with_detail(serde_json::json!({
                "stderr": stderr,
                "status": output.status.to_string(),
            }));
        }
        return Err(err.into());
    }

    serde_json::from_slice(&output.stdout).map_err(|err| {
        service_error(
            "invalid_output_json",
            format!(
                "Service '{}.{}' returned invalid JSON: {}",
                service.id, invocation.method, err
            ),
            &service.id,
            Some(&invocation.method),
        )
        .with_detail(serde_json::json!({
            "stdout": String::from_utf8_lossy(&output.stdout).trim(),
        }))
        .into()
    })
}

fn command_for_service(service: &Service) -> Result<Command> {
    if let Some(exec) = legacy_exec(service) {
        return Ok(command_for_exec(exec));
    }

    let entry = service_entry(service).ok_or_else(|| missing_entry_error(service))?;
    let mut command = Command::new(python_for_service(service));
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

fn command_for_exec(exec: &str) -> Command {
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
