use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::events::{EventSource, OperationEvent};

use super::model::{Service, ServiceRuntimeKind};

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

pub async fn invoke_service(
    home: &Path,
    id: &str,
    invocation: ServiceInvocation,
) -> Result<ServiceInvocationResult> {
    let op = OperationEvent::new(home, EventSource::Core, "service.invoke")
        .attr("service_id", id)
        .attr("method", &invocation.method);
    op.emit_start();

    let result = (|| {
        let service = super::get_service(home, id)?
            .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", id))?;
        validate_method(&service, &invocation.method)?;

        let output = match service.runtime.kind {
            ServiceRuntimeKind::Command => invoke_command_service(&service, &invocation)?,
            ServiceRuntimeKind::Builtin => {
                bail!(
                    "Builtin service '{}' does not implement direct invocation yet",
                    id
                )
            }
            ServiceRuntimeKind::Http
            | ServiceRuntimeKind::Daemon
            | ServiceRuntimeKind::External => {
                bail!(
                    "Service runtime '{:?}' does not support invoke yet",
                    service.runtime.kind
                )
            }
        };

        Ok(ServiceInvocationResult {
            service_id: service.id,
            method: invocation.method,
            output,
        })
    })();

    op.emit_result(&result);
    result
}

fn validate_method(service: &Service, method: &str) -> Result<()> {
    if service.methods.iter().any(|entry| entry.name == method) {
        return Ok(());
    }

    bail!(
        "Service '{}' does not declare method '{}'",
        service.id,
        method
    )
}

fn invoke_command_service(
    service: &Service,
    invocation: &ServiceInvocation,
) -> Result<serde_json::Value> {
    let exec = service
        .runtime
        .exec
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Service '{}' has no runtime exec. Install it first or define runtime.exec.",
                service.id
            )
        })?;

    let request = serde_json::json!({
        "method": invocation.method,
        "input": invocation.input,
        "context": invocation.context,
    });

    let mut child = command_for_exec(exec)
        .current_dir(&service.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to start service command '{}'", exec))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for service '{}'", service.id))?;
        serde_json::to_writer(&mut *stdin, &request)
            .with_context(|| format!("Failed to write request to service '{}'", service.id))?;
        stdin
            .write_all(b"\n")
            .with_context(|| format!("Failed to finalize request for service '{}'", service.id))?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed to read output from service '{}'", service.id))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "Service '{}' command failed with status {}{}",
            service.id,
            output.status,
            if stderr.is_empty() {
                String::new()
            } else {
                format!(": {}", stderr)
            }
        );
    }

    serde_json::from_slice(&output.stdout)
        .with_context(|| format!("Service '{}' returned invalid JSON", service.id))
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
