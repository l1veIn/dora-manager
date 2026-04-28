use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use dm_core::service::{
    Service, ServiceInvocation, ServiceInvocationError, ServiceInvocationResult, ServiceMethod,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

mod runtime;

const IDLE_EXIT_SECS: u64 = 300;
const IDLE_CHECK_SECS: u64 = 30;
const DEFAULT_SERVICE_TIMEOUT_MS: u64 = 10_000;

#[tokio::main]
async fn main() -> Result<()> {
    let home = dm_core::config::resolve_home(None)?;
    std::fs::create_dir_all(&home)
        .with_context(|| format!("failed to create dm home at {}", home.display()))?;

    let socket_path = home.join("service.sock");
    remove_stale_socket(&socket_path)?;
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind {}", socket_path.display()))?;

    runtime::start_service_runtime_reaper();

    let last_invoke = Arc::new(AtomicU64::new(timestamp_secs()));
    spawn_idle_exit(last_invoke.clone());

    loop {
        let (stream, _) = listener.accept().await?;
        let home = home.clone();
        let last_invoke = last_invoke.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(&home, stream, last_invoke).await {
                eprintln!("dm-serviced connection error: {err:#}");
            }
        });
    }
}

fn remove_stale_socket(socket_path: &Path) -> Result<()> {
    match std::fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| {
            format!(
                "failed to remove stale service socket at {}",
                socket_path.display()
            )
        }),
    }
}

fn spawn_idle_exit(last_invoke: Arc<AtomicU64>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(IDLE_CHECK_SECS)).await;
            let elapsed = timestamp_secs().saturating_sub(last_invoke.load(Ordering::Relaxed));
            if elapsed > IDLE_EXIT_SECS {
                std::process::exit(0);
            }
        }
    });
}

async fn handle_connection(
    home: &Path,
    stream: UnixStream,
    last_invoke: Arc<AtomicU64>,
) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    if reader.read_line(&mut line).await? == 0 {
        return Ok(());
    }

    last_invoke.store(timestamp_secs(), Ordering::Relaxed);
    let response = handle_request(home, line.trim()).await;
    write_half.write_all(response.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    write_half.flush().await?;
    Ok(())
}

async fn handle_request(home: &Path, line: &str) -> String {
    let result = async {
        let value: serde_json::Value =
            serde_json::from_str(line).context("invalid request JSON")?;
        let service_id = value
            .get("service_id")
            .and_then(|value| value.as_str())
            .context("missing string field 'service_id'")?;
        let method = value
            .get("method")
            .and_then(|value| value.as_str())
            .context("missing string field 'method'")?;
        let invocation = ServiceInvocation {
            method: method.to_string(),
            input: value
                .get("input")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            context: value.get("context").cloned(),
        };

        invoke_service_local(home, service_id, invocation).await
    }
    .await;

    match result {
        Ok(result) => serde_json::to_string(&result).unwrap_or_else(fallback_error),
        Err(err) => {
            if let Some(invocation_err) = err.downcast_ref::<ServiceInvocationError>() {
                serde_json::to_string(invocation_err).unwrap_or_else(fallback_error)
            } else {
                serde_json::json!({
                    "code": "service_invoke_failed",
                    "message": err.to_string(),
                })
                .to_string()
            }
        }
    }
}

fn fallback_error(_: serde_json::Error) -> String {
    r#"{"code":"serialization_failed","message":"failed to serialize response"}"#.to_string()
}

async fn invoke_service_local(
    home: &Path,
    id: &str,
    invocation: ServiceInvocation,
) -> Result<ServiceInvocationResult> {
    let service = dm_core::service::get_service(home, id)?.ok_or_else(|| {
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

    let timeout = Duration::from_millis(
        service
            .timeout_ms
            .or(service.runtime.timeout_ms)
            .unwrap_or(DEFAULT_SERVICE_TIMEOUT_MS),
    );
    let output = runtime::SERVICE_RUNTIME
        .invoke(
            &service,
            &invocation.method,
            invocation.input.clone(),
            invocation.context.clone(),
            timeout,
        )
        .await?;

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

fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
