use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};

use crate::faas::{HealthResponse, InvokeRequest, InvokeResponse};
use crate::AppState;

pub async fn list_functions(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let functions = state.faas.functions.lock().await;
    let list: Vec<serde_json::Value> = functions
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "name": f.name,
                "version": f.version,
                "description": f.description,
                "methods": f.methods.iter().map(|m| serde_json::json!({
                    "name": m.name,
                    "description": m.description,
                    "events": m.events,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Json(list)
}

pub async fn get_function(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let functions = state.faas.functions.lock().await;
    match functions.iter().find(|f| f.id == id) {
        Some(func) => Json(serde_json::json!({
            "id": func.id,
            "name": func.name,
            "version": func.version,
            "description": func.description,
            "entry": func.entry,
            "methods": func.methods,
            "runtime": {
                "max_workers": func.runtime.max_workers,
                "idle_timeout_secs": func.runtime.idle_timeout_secs,
            },
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "function not found"})),
        )
            .into_response(),
    }
}

pub async fn invoke_function(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<InvokeRequest>,
) -> impl IntoResponse {
    let func = {
        let functions = state.faas.functions.lock().await;
        match functions.iter().find(|f| f.id == id) {
            Some(f) => f.clone(),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "function not found"})),
                )
                    .into_response();
            }
        }
    };

    let has_method = func.methods.iter().any(|m| m.name == req.method);
    if !has_method {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("method '{}' not found", req.method),
                "code": "method_not_found",
            })),
        )
            .into_response();
    }

    let pool = state.faas.pools.for_function(&func).await;

    let mut worker = match pool.acquire().await {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let request = serde_json::json!({
        "method": req.method,
        "input": req.input,
    });

    match worker.call(&request).await {
        Ok(output) => Json(InvokeResponse { output }).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn faas_health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        uptime_secs: state.faas.start_time.elapsed().as_secs(),
        functions: state.faas.functions.lock().await.len(),
        workers: state.faas.pools.total_workers().await,
    })
}
