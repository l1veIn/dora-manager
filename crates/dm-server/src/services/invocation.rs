use serde_json::Value;

use crate::services::message::{normalize_payload, MessageFilter, MessageService};
use crate::state::{AppState, MessageNotification};

pub type InvocationError = dm_core::service::ServiceInvocationError;
pub type InvocationResult = dm_core::service::ServiceInvocationResult;

pub async fn invoke_server_service(
    state: &AppState,
    id: &str,
    invocation: dm_core::service::ServiceInvocation,
) -> Result<Option<InvocationResult>, InvocationError> {
    match id {
        "message" => invoke_message_service(state, invocation).map(Some),
        _ => Ok(None),
    }
}

fn invoke_message_service(
    state: &AppState,
    invocation: dm_core::service::ServiceInvocation,
) -> Result<InvocationResult, InvocationError> {
    let run_id = invocation
        .context
        .as_ref()
        .and_then(|context| context.get("run_id"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            invocation_error(
                "context_required",
                "Service 'message' requires context.run_id",
                "message",
                Some(&invocation.method),
            )
        })?
        .to_string();

    let output = match invocation.method.as_str() {
        "send" => invoke_message_send(state, &run_id, &invocation.input)?,
        "list" => invoke_message_list(state, &run_id, &invocation.input)?,
        "snapshots" => invoke_message_snapshots(state, &run_id)?,
        method => {
            return Err(invocation_error(
                "method_not_found",
                format!("Service 'message' does not declare method '{}'", method),
                "message",
                Some(method),
            ));
        }
    };

    Ok(InvocationResult {
        service_id: "message".to_string(),
        method: invocation.method,
        output,
    })
}

fn invoke_message_send(
    state: &AppState,
    run_id: &str,
    input: &Value,
) -> Result<Value, InvocationError> {
    let from = required_string(input, "from", "message", "send")?;
    let tag = required_string(input, "tag", "message", "send")?;
    let payload = input.get("payload").cloned().ok_or_else(|| {
        invocation_error(
            "input_validation_failed",
            "Service 'message.send' input requires 'payload'",
            "message",
            Some("send"),
        )
    })?;
    let timestamp = input
        .get("timestamp")
        .and_then(Value::as_i64)
        .unwrap_or_else(crate::services::now_ts);
    let payload = normalize_payload(&tag, payload).map_err(|err| {
        invocation_error(
            "input_validation_failed",
            format!("Service 'message.send' input failed validation: {}", err),
            "message",
            Some("send"),
        )
    })?;
    let service = MessageService::open(&state.home, run_id).map_err(|err| {
        invocation_error(
            "invoke_failed",
            format!("Service 'message.send' failed: {}", err),
            "message",
            Some("send"),
        )
    })?;
    let seq = service
        .push(&from, &tag, &payload, timestamp)
        .map_err(|err| {
            invocation_error(
                "invoke_failed",
                format!("Service 'message.send' failed: {}", err),
                "message",
                Some("send"),
            )
        })?;
    let _ = state.messages.send(MessageNotification {
        run_id: run_id.to_string(),
        seq,
        from,
        tag,
    });

    Ok(serde_json::json!({ "seq": seq }))
}

fn invoke_message_list(
    state: &AppState,
    run_id: &str,
    input: &Value,
) -> Result<Value, InvocationError> {
    let service = MessageService::open(&state.home, run_id).map_err(|err| {
        invocation_error(
            "invoke_failed",
            format!("Service 'message.list' failed: {}", err),
            "message",
            Some("list"),
        )
    })?;
    let response = service
        .list(&MessageFilter {
            after_seq: optional_i64(input, "after_seq", "message", "list")?,
            before_seq: optional_i64(input, "before_seq", "message", "list")?,
            from: optional_string_array(input, "from", "message", "list")?,
            tag: optional_string_array(input, "tag", "message", "list")?,
            target_to: None,
            limit: optional_usize(input, "limit", "message", "list")?,
            desc: input.get("desc").and_then(Value::as_bool),
        })
        .map_err(|err| {
            invocation_error(
                "invoke_failed",
                format!("Service 'message.list' failed: {}", err),
                "message",
                Some("list"),
            )
        })?;
    serde_json::to_value(response).map_err(|err| {
        invocation_error(
            "invalid_output_json",
            format!("Service 'message.list' returned invalid JSON: {}", err),
            "message",
            Some("list"),
        )
    })
}

fn invoke_message_snapshots(state: &AppState, run_id: &str) -> Result<Value, InvocationError> {
    let service = MessageService::open(&state.home, run_id).map_err(|err| {
        invocation_error(
            "invoke_failed",
            format!("Service 'message.snapshots' failed: {}", err),
            "message",
            Some("snapshots"),
        )
    })?;
    let snapshots = service.snapshots().map_err(|err| {
        invocation_error(
            "invoke_failed",
            format!("Service 'message.snapshots' failed: {}", err),
            "message",
            Some("snapshots"),
        )
    })?;

    Ok(serde_json::json!({ "snapshots": snapshots }))
}

pub fn invocation_error(
    code: impl Into<String>,
    message: impl Into<String>,
    service_id: &str,
    method: Option<&str>,
) -> InvocationError {
    InvocationError {
        code: code.into(),
        message: message.into(),
        service_id: Some(service_id.to_string()),
        method: method.map(ToString::to_string),
        detail: None,
    }
}

fn required_string(
    input: &Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<String, InvocationError> {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            invocation_error(
                "input_validation_failed",
                format!(
                    "Service '{}.{}' input requires '{}'",
                    service_id, method, key
                ),
                service_id,
                Some(method),
            )
        })
}

fn optional_i64(
    input: &Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<Option<i64>, InvocationError> {
    let Some(value) = input.get(key) else {
        return Ok(None);
    };
    value.as_i64().map(Some).ok_or_else(|| {
        invocation_error(
            "input_validation_failed",
            format!(
                "Service '{}.{}' input field '{}' must be an integer",
                service_id, method, key
            ),
            service_id,
            Some(method),
        )
    })
}

fn optional_usize(
    input: &Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<Option<usize>, InvocationError> {
    let Some(value) = input.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_u64() else {
        return Err(invocation_error(
            "input_validation_failed",
            format!(
                "Service '{}.{}' input field '{}' must be a non-negative integer",
                service_id, method, key
            ),
            service_id,
            Some(method),
        ));
    };
    usize::try_from(value).map(Some).map_err(|_| {
        invocation_error(
            "input_validation_failed",
            format!(
                "Service '{}.{}' input field '{}' is too large",
                service_id, method, key
            ),
            service_id,
            Some(method),
        )
    })
}

fn optional_string_array(
    input: &Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<Option<Vec<String>>, InvocationError> {
    let Some(value) = input.get(key) else {
        return Ok(None);
    };
    let Some(items) = value.as_array() else {
        return Err(invocation_error(
            "input_validation_failed",
            format!(
                "Service '{}.{}' input field '{}' must be an array of strings",
                service_id, method, key
            ),
            service_id,
            Some(method),
        ));
    };
    items
        .iter()
        .map(|item| {
            item.as_str().map(ToString::to_string).ok_or_else(|| {
                invocation_error(
                    "input_validation_failed",
                    format!(
                        "Service '{}.{}' input field '{}' must be an array of strings",
                        service_id, method, key
                    ),
                    service_id,
                    Some(method),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
