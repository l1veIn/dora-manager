use axum::extract::{Path, State};
use axum::http::header::{self, HeaderValue};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::handlers::err;
use crate::services::invocation::invoke_server_service;
use crate::state::AppState;

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
) -> Response {
    invoke_service_request(
        &state,
        &id,
        dm_core::service::ServiceInvocation {
            method: req.method,
            input: req.input,
            context: req.context,
        },
    )
    .await
}

/// POST /api/runs/:run_id/services/:id/invoke
#[utoipa::path(post, path = "/api/runs/{run_id}/services/{id}/invoke", params(("run_id" = String, Path, description = "Run ID"), ("id" = String, Path, description = "Service ID")), request_body = InvokeServiceRequest, responses((status = 200, description = "Run-scoped service invocation result")))]
pub async fn invoke_run_service(
    State(state): State<AppState>,
    Path((run_id, id)): Path<(String, String)>,
    Json(req): Json<InvokeServiceRequest>,
) -> Response {
    let mut context = req.context.unwrap_or_else(|| serde_json::json!({}));
    if !context.is_object() {
        context = serde_json::json!({});
    }
    if let Some(object) = context.as_object_mut() {
        object.insert("run_id".to_string(), serde_json::Value::String(run_id));
    }

    invoke_service_request(
        &state,
        &id,
        dm_core::service::ServiceInvocation {
            method: req.method,
            input: req.input,
            context: Some(context),
        },
    )
    .await
}

async fn invoke_service_request(
    state: &AppState,
    id: &str,
    invocation: dm_core::service::ServiceInvocation,
) -> Response {
    match invoke_server_service(state, id, invocation.clone()).await {
        Ok(Some(result)) => return Json(result).into_response(),
        Ok(None) => {}
        Err(error) => return service_invocation_error_response(error).into_response(),
    }

    match dm_core::service::invoke_service(&state.home, id, invocation).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => service_invoke_err(e).into_response(),
    }
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
