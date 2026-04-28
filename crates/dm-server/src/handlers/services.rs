use axum::extract::{Path, State};
use axum::http::header::{self, HeaderValue};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::handlers::err;
use crate::services::message::{MessageFilter, MessageService};
use crate::state::AppState;
use crate::MessageNotification;

use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceErrorResponse {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

/// GET /api/services
#[utoipa::path(get, path = "/api/services", responses((status = 200, description = "List of available services")))]
pub async fn list_services(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::service::list_services(&state.home) {
        Ok(services) => Json(services).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/services/:id
#[utoipa::path(get, path = "/api/services/{id}", params(("id" = String, Path, description = "Service ID")), responses((status = 200, description = "Service details")))]
pub async fn service_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::service::service_status(&state.home, &id) {
        Ok(Some(entry)) => Json(entry).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, format!("Service '{}' not found", id)).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateServiceRequest {
    pub id: String,
    #[serde(default)]
    pub description: String,
}

/// POST /api/services/create
#[utoipa::path(post, path = "/api/services/create", request_body = CreateServiceRequest, responses((status = 200, description = "Created service")))]
pub async fn create_service(
    State(state): State<AppState>,
    Json(req): Json<CreateServiceRequest>,
) -> impl IntoResponse {
    match dm_core::service::create_service(&state.home, &req.id, &req.description) {
        Ok(entry) => Json(entry).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct InstallServiceRequest {
    pub id: String,
}

/// POST /api/services/install
#[utoipa::path(post, path = "/api/services/install", request_body = InstallServiceRequest, responses((status = 200, description = "Installed service")))]
pub async fn install_service(
    State(state): State<AppState>,
    Json(req): Json<InstallServiceRequest>,
) -> impl IntoResponse {
    match dm_core::service::install_service(&state.home, &req.id).await {
        Ok(entry) => Json(entry).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct InvokeServiceRequest {
    pub method: String,
    #[serde(default)]
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// POST /api/services/:id/invoke
#[utoipa::path(post, path = "/api/services/{id}/invoke", params(("id" = String, Path, description = "Service ID")), request_body = InvokeServiceRequest, responses((status = 200, description = "Service invocation result")))]
pub async fn invoke_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<InvokeServiceRequest>,
) -> impl IntoResponse {
    if id == "message" {
        return match invoke_message_service(&state, req) {
            Ok(result) => Json(result).into_response(),
            Err(error) => service_invocation_error_response(error).into_response(),
        };
    }

    match dm_core::service::invoke_service(
        &state.home,
        &id,
        dm_core::service::ServiceInvocation {
            method: req.method,
            input: req.input,
            context: req.context,
        },
    )
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(e) => service_invoke_err(e).into_response(),
    }
}

fn invoke_message_service(
    state: &AppState,
    req: InvokeServiceRequest,
) -> Result<dm_core::service::ServiceInvocationResult, dm_core::service::ServiceInvocationError> {
    let run_id = req
        .context
        .as_ref()
        .and_then(|context| context.get("run_id"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            service_invocation_error(
                "context_required",
                "Service 'message' requires context.run_id",
                "message",
                Some(&req.method),
            )
        })?
        .to_string();

    let output = match req.method.as_str() {
        "send" => invoke_message_send(state, &run_id, &req.input)?,
        "list" => invoke_message_list(state, &run_id, &req.input)?,
        "snapshots" => invoke_message_snapshots(state, &run_id)?,
        method => {
            return Err(service_invocation_error(
                "method_not_found",
                format!("Service 'message' does not declare method '{}'", method),
                "message",
                Some(method),
            ));
        }
    };

    Ok(dm_core::service::ServiceInvocationResult {
        service_id: "message".to_string(),
        method: req.method,
        output,
    })
}

fn invoke_message_send(
    state: &AppState,
    run_id: &str,
    input: &serde_json::Value,
) -> Result<serde_json::Value, dm_core::service::ServiceInvocationError> {
    let from = required_string(input, "from", "message", "send")?;
    let tag = required_string(input, "tag", "message", "send")?;
    let payload = input.get("payload").cloned().ok_or_else(|| {
        service_invocation_error(
            "input_validation_failed",
            "Service 'message.send' input requires 'payload'",
            "message",
            Some("send"),
        )
    })?;
    let timestamp = input
        .get("timestamp")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_else(crate::services::now_ts);
    let payload = crate::handlers::messages::normalize_payload(&tag, payload).map_err(|err| {
        service_invocation_error(
            "input_validation_failed",
            format!("Service 'message.send' input failed validation: {}", err),
            "message",
            Some("send"),
        )
    })?;
    let service = MessageService::open(&state.home, run_id).map_err(|err| {
        service_invocation_error(
            "invoke_failed",
            format!("Service 'message.send' failed: {}", err),
            "message",
            Some("send"),
        )
    })?;
    let seq = service
        .push(&from, &tag, &payload, timestamp)
        .map_err(|err| {
            service_invocation_error(
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
    input: &serde_json::Value,
) -> Result<serde_json::Value, dm_core::service::ServiceInvocationError> {
    let service = MessageService::open(&state.home, run_id).map_err(|err| {
        service_invocation_error(
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
            desc: input.get("desc").and_then(serde_json::Value::as_bool),
        })
        .map_err(|err| {
            service_invocation_error(
                "invoke_failed",
                format!("Service 'message.list' failed: {}", err),
                "message",
                Some("list"),
            )
        })?;
    serde_json::to_value(response).map_err(|err| {
        service_invocation_error(
            "invalid_output_json",
            format!("Service 'message.list' returned invalid JSON: {}", err),
            "message",
            Some("list"),
        )
    })
}

fn invoke_message_snapshots(
    state: &AppState,
    run_id: &str,
) -> Result<serde_json::Value, dm_core::service::ServiceInvocationError> {
    let service = MessageService::open(&state.home, run_id).map_err(|err| {
        service_invocation_error(
            "invoke_failed",
            format!("Service 'message.snapshots' failed: {}", err),
            "message",
            Some("snapshots"),
        )
    })?;
    let snapshots = service.snapshots().map_err(|err| {
        service_invocation_error(
            "invoke_failed",
            format!("Service 'message.snapshots' failed: {}", err),
            "message",
            Some("snapshots"),
        )
    })?;

    Ok(serde_json::json!({ "snapshots": snapshots }))
}

fn service_invoke_err(error: anyhow::Error) -> impl IntoResponse {
    if let Some(invocation_error) = error.downcast_ref::<dm_core::service::ServiceInvocationError>()
    {
        return service_invocation_error_response(invocation_error.clone()).into_response();
    }

    (
        StatusCode::BAD_REQUEST,
        Json(ServiceErrorResponse {
            error: error.to_string(),
            code: "invoke_failed".to_string(),
            service_id: None,
            method: None,
            detail: None,
        }),
    )
        .into_response()
}

fn service_invocation_error_response(
    invocation_error: dm_core::service::ServiceInvocationError,
) -> impl IntoResponse {
    let status = match invocation_error.code.as_str() {
        "service_not_found" => StatusCode::NOT_FOUND,
        _ => StatusCode::BAD_REQUEST,
    };
    (
        status,
        Json(ServiceErrorResponse {
            error: invocation_error.message,
            code: invocation_error.code,
            service_id: invocation_error.service_id,
            method: invocation_error.method,
            detail: invocation_error.detail,
        }),
    )
}

fn service_invocation_error(
    code: impl Into<String>,
    message: impl Into<String>,
    service_id: &str,
    method: Option<&str>,
) -> dm_core::service::ServiceInvocationError {
    dm_core::service::ServiceInvocationError {
        code: code.into(),
        message: message.into(),
        service_id: Some(service_id.to_string()),
        method: method.map(ToString::to_string),
        detail: None,
    }
}

fn required_string(
    input: &serde_json::Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<String, dm_core::service::ServiceInvocationError> {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            service_invocation_error(
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
    input: &serde_json::Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<Option<i64>, dm_core::service::ServiceInvocationError> {
    let Some(value) = input.get(key) else {
        return Ok(None);
    };
    value.as_i64().map(Some).ok_or_else(|| {
        service_invocation_error(
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
    input: &serde_json::Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<Option<usize>, dm_core::service::ServiceInvocationError> {
    let Some(value) = input.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_u64() else {
        return Err(service_invocation_error(
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
        service_invocation_error(
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
    input: &serde_json::Value,
    key: &str,
    service_id: &str,
    method: &str,
) -> Result<Option<Vec<String>>, dm_core::service::ServiceInvocationError> {
    let Some(value) = input.get(key) else {
        return Ok(None);
    };
    let Some(items) = value.as_array() else {
        return Err(service_invocation_error(
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
                service_invocation_error(
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

#[derive(Deserialize, ToSchema)]
pub struct ImportServiceRequest {
    /// Local path or git URL
    pub source: String,
    /// Override service id (default: inferred from service.json or source basename)
    pub id: Option<String>,
}

/// POST /api/services/import
#[utoipa::path(post, path = "/api/services/import", request_body = ImportServiceRequest, responses((status = 200, description = "Imported service")))]
pub async fn import_service(
    State(state): State<AppState>,
    Json(req): Json<ImportServiceRequest>,
) -> impl IntoResponse {
    let is_url = req.source.starts_with("https://") || req.source.starts_with("http://");
    let source_path = std::path::Path::new(&req.source);
    let abs_path = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        state.home.join(source_path)
    };
    let inferred_id = req
        .id
        .unwrap_or_else(|| infer_service_import_id(&abs_path, &req.source, is_url));

    let result = if is_url {
        dm_core::service::import_git(&state.home, &inferred_id, &req.source).await
    } else {
        dm_core::service::import_local(&state.home, &inferred_id, &abs_path)
    };

    match result {
        Ok(service) => Json(service).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UninstallServiceRequest {
    pub id: String,
}

/// POST /api/services/uninstall
#[utoipa::path(post, path = "/api/services/uninstall", request_body = UninstallServiceRequest, responses((status = 200, description = "Uninstall result")))]
pub async fn uninstall_service(
    State(state): State<AppState>,
    Json(req): Json<UninstallServiceRequest>,
) -> impl IntoResponse {
    match dm_core::service::uninstall_service(&state.home, &req.id) {
        Ok(()) => {
            Json(serde_json::json!({ "message": format!("Uninstalled service '{}'", req.id) }))
                .into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn service_readme(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Ok(content) = dm_core::service::get_service_readme(&state.home, &id) {
        return content.into_response();
    }

    (format!("No README found locally for '{}'", id),).into_response()
}

/// GET /api/services/:id/config
#[utoipa::path(get, path = "/api/services/{id}/config", params(("id" = String, Path, description = "Service ID")), responses((status = 200, description = "Service configuration")))]
pub async fn get_service_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::service::get_service_config(&state.home, &id) {
        Ok(config) => Json(config).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/services/:id/config
#[utoipa::path(post, path = "/api/services/{id}/config", params(("id" = String, Path, description = "Service ID")), responses((status = 200, description = "Config saved")))]
pub async fn save_service_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> impl IntoResponse {
    match dm_core::service::save_service_config(&state.home, &id, &config) {
        Ok(()) => Json(serde_json::json!({ "message": "Config saved" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// GET /api/services/:id/files
pub async fn get_service_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match dm_core::service::git_like_file_tree(&state.home, &id) {
        Ok(files) => Json(files).into_response(),
        Err(e) => service_file_err(e, &id).into_response(),
    }
}

/// GET /api/services/:id/files/{*path}
pub async fn get_service_file_content(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(String, String)>,
) -> impl IntoResponse {
    match dm_core::service::read_service_file(&state.home, &id, &file_path) {
        Ok(content) => content.into_response(),
        Err(e) => service_file_err(e, &id).into_response(),
    }
}

/// GET /api/services/:id/artifacts/{*path}
pub async fn serve_service_artifact_file(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(String, String)>,
) -> impl IntoResponse {
    match dm_core::service::read_service_file_bytes(&state.home, &id, &file_path) {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
            let mut resp = bytes.into_response();
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            resp
        }
        Err(e) => service_file_err(e, &id).into_response(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct OpenServiceRequest {
    pub target: String,
}

/// POST /api/services/:id/open
#[utoipa::path(post, path = "/api/services/{id}/open", params(("id" = String, Path, description = "Service ID")), request_body = OpenServiceRequest, responses((status = 200, description = "Opened service in external tool")))]
pub async fn open_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<OpenServiceRequest>,
) -> impl IntoResponse {
    let Some(service_path) = dm_core::service::resolve_service_dir(&state.home, &id) else {
        return (StatusCode::NOT_FOUND, format!("Service '{}' not found", id)).into_response();
    };

    let result = match req.target.as_str() {
        "finder" => Command::new("open").arg(&service_path).status(),
        "terminal" => Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&service_path)
            .status(),
        "vscode" => Command::new("open")
            .arg("-a")
            .arg("Visual Studio Code")
            .arg(&service_path)
            .status(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Unsupported open target '{}'", req.target),
            )
                .into_response();
        }
    };

    match result {
        Ok(status) if status.success() => Json(serde_json::json!({
            "message": format!("Opened '{}' in {}", id, req.target)
        }))
        .into_response(),
        Ok(status) => (
            StatusCode::BAD_REQUEST,
            format!("Failed to open '{}': launcher exited with {}", id, status),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("Failed to open '{}': {}", id, e),
        )
            .into_response(),
    }
}

fn service_file_err(e: anyhow::Error, id: &str) -> (StatusCode, String) {
    let message = e.to_string();
    if message.contains("Invalid service file path") {
        return (StatusCode::BAD_REQUEST, message);
    }
    if message.contains("does not exist")
        || message.contains("No such file or directory")
        || message == format!("Service '{}' not found", id)
    {
        return (StatusCode::NOT_FOUND, message);
    }

    (StatusCode::INTERNAL_SERVER_ERROR, message)
}

fn infer_service_import_id(source_path: &std::path::Path, source: &str, is_url: bool) -> String {
    if is_url {
        return source
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or("unknown")
            .to_string();
    }

    let manifest_path = source_path.join("service.json");
    if let Ok(content) = std::fs::read_to_string(manifest_path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
                return id.to_string();
            }
        }
    }

    source_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}
