use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::AppState;

/// GET /api/dataflows
pub async fn list_dataflows(State(state): State<AppState>) -> impl IntoResponse {
    match dm_core::dataflow::list(&state.home) {
        Ok(result) => Json(result).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// GET /api/dataflows/:name
pub async fn get_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::get(&state.home, &name) {
        Ok(content) => Json(serde_json::json!({ "yaml": content })).into_response(),
        Err(e) => {
            if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    return (StatusCode::NOT_FOUND, format!("Dataflow '{}' not found", name))
                        .into_response();
                }
            }
            err(e).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct SaveDataflowRequest {
    pub yaml: String,
}

/// POST /api/dataflows/:name
pub async fn save_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SaveDataflowRequest>,
) -> impl IntoResponse {
    match dm_core::dataflow::save(&state.home, &name, &req.yaml) {
        Ok(()) => Json(serde_json::json!({ "message": "Saved successfully" })).into_response(),
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/dataflows/:name/delete
pub async fn delete_dataflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match dm_core::dataflow::delete(&state.home, &name) {
        Ok(()) => Json(serde_json::json!({ "message": "Deleted successfully" })).into_response(),
        Err(e) => {
            if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    return (StatusCode::NOT_FOUND, format!("Dataflow '{}' not found", name))
                        .into_response();
                }
            }
            err(e).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct RunDataflowRequest {
    pub yaml: String,
}

/// POST /api/dataflow/start
pub async fn start_dataflow(
    State(state): State<AppState>,
    Json(req): Json<RunDataflowRequest>,
) -> impl IntoResponse {
    if !dm_core::is_runtime_running(&state.home, false).await {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "Dora runtime is not running. Call POST /api/up first."
            })),
        )
            .into_response();
    }

    let run_dir = dm_core::config::resolve_home(None)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("run");
    let _ = std::fs::create_dir_all(&run_dir);

    let tmp_path = run_dir.join("_web_dataflow.yml");
    if let Err(e) = std::fs::write(&tmp_path, &req.yaml) {
        return err(e).into_response();
    }

    let transpiled = match dm_core::dataflow::transpile_graph(&state.home, &tmp_path) {
        Ok(v) => v,
        Err(e) => return err(e).into_response(),
    };

    let transpiled_path = run_dir.join("_web_dataflow_transpiled.yml");
    let transpiled_str = match serde_yaml::to_string(&transpiled) {
        Ok(s) => s,
        Err(e) => return err(e).into_response(),
    };
    if let Err(e) = std::fs::write(&transpiled_path, &transpiled_str) {
        return err(e).into_response();
    }

    let dora_bin = match dm_core::dora::active_dora_bin(&state.home) {
        Ok(bin) => bin,
        Err(e) => return err(e).into_response(),
    };

    let result = tokio::process::Command::new(&dora_bin)
        .args(["start", &transpiled_path.to_string_lossy(), "--detach"])
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Json(serde_json::json!({
                "status": "started",
                "message": stdout,
            }))
            .into_response()
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            (StatusCode::INTERNAL_SERVER_ERROR, stderr).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

/// POST /api/dataflow/stop
pub async fn stop_dataflow(State(state): State<AppState>) -> impl IntoResponse {
    let dora_bin = match dm_core::dora::active_dora_bin(&state.home) {
        Ok(bin) => bin,
        Err(e) => return err(e).into_response(),
    };

    let result = tokio::process::Command::new(&dora_bin)
        .arg("stop")
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Json(serde_json::json!({
                "status": "stopped",
                "message": stdout,
            }))
            .into_response()
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            (StatusCode::INTERNAL_SERVER_ERROR, stderr).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}
