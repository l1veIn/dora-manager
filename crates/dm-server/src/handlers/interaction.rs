use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::handlers::err;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InteractionState {
    #[serde(default)]
    pub displays: Vec<DisplayEntry>,
    #[serde(default)]
    pub inputs: Vec<InputBinding>,
    #[serde(default)]
    pub next_event_seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayEntry {
    pub node_id: String,
    pub label: String,
    pub file: String,
    pub render: String,
    pub tail: bool,
    pub max_lines: usize,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputBinding {
    pub node_id: String,
    pub label: String,
    #[serde(default)]
    pub widgets: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub current_values: BTreeMap<String, serde_json::Value>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    pub seq: i64,
    pub node_id: String,
    pub output_id: String,
    pub value: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputState {
    #[serde(default)]
    pub events: Vec<InputEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionSnapshot {
    #[serde(default)]
    pub displays: Vec<DisplayEntry>,
    #[serde(default)]
    pub inputs: Vec<InputBinding>,
}

#[derive(Debug, Deserialize)]
pub struct DisplayUpdateRequest {
    pub node_id: String,
    pub label: String,
    pub file: String,
    pub render: String,
    pub tail: Option<bool>,
    pub max_lines: Option<usize>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterInputRequest {
    pub node_id: String,
    pub label: Option<String>,
    pub widgets: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct EmitInputEventRequest {
    pub node_id: String,
    pub output_id: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ClaimEventsParams {
    pub since: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ClaimEventsResponse {
    pub events: Vec<InputEvent>,
    pub next_seq: i64,
}

pub async fn get_interaction(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    match load_state(&state.home, &run_id) {
        Ok(snapshot) => Json(InteractionSnapshot {
            displays: snapshot.displays,
            inputs: snapshot.inputs,
        })
        .into_response(),
        Err(e) => err(e).into_response(),
    }
}

pub async fn post_display(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<DisplayUpdateRequest>,
) -> impl IntoResponse {
    match mutate_state(&state.home, &run_id, |state| {
        let entry = DisplayEntry {
            node_id: req.node_id,
            label: req.label,
            file: normalize_relative_path(&req.file)?,
            render: req.render,
            tail: req.tail.unwrap_or(true),
            max_lines: req.max_lines.unwrap_or(500),
            updated_at: req.timestamp.unwrap_or_else(now_ts),
        };

        if let Some(existing) = state.displays.iter_mut().find(|item| item.node_id == entry.node_id) {
            *existing = entry;
        } else {
            state.displays.push(entry);
        }
        state.displays.sort_by(|a, b| a.label.cmp(&b.label));
        Ok(())
    }) {
        Ok(snapshot) => Json(InteractionSnapshot {
            displays: snapshot.displays,
            inputs: snapshot.inputs,
        })
        .into_response(),
        Err(e) => err(e).into_response(),
    }
}

pub async fn register_input(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<RegisterInputRequest>,
) -> impl IntoResponse {
    match mutate_state(&state.home, &run_id, |state| {
        let updated_at = now_ts();
        if let Some(existing) = state.inputs.iter_mut().find(|item| item.node_id == req.node_id) {
            existing.label = req.label.clone().unwrap_or_else(|| req.node_id.clone());
            existing.widgets = req.widgets.clone();
            existing.updated_at = updated_at;
        } else {
            state.inputs.push(InputBinding {
                node_id: req.node_id.clone(),
                label: req.label.clone().unwrap_or_else(|| req.node_id.clone()),
                widgets: req.widgets.clone(),
                current_values: BTreeMap::new(),
                updated_at,
            });
        }
        state.inputs.sort_by(|a, b| a.label.cmp(&b.label));
        Ok(())
    }) {
        Ok(snapshot) => Json(InteractionSnapshot {
            displays: snapshot.displays,
            inputs: snapshot.inputs,
        })
        .into_response(),
        Err(e) => err(e).into_response(),
    }
}

pub async fn emit_input_event(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<EmitInputEventRequest>,
) -> impl IntoResponse {
    let result = (|| -> anyhow::Result<InteractionSnapshot> {
        let mut interaction = load_state(&state.home, &run_id)?;
        let Some(binding) = interaction
            .inputs
            .iter_mut()
            .find(|item| item.node_id == req.node_id)
        else {
            return Err(anyhow::anyhow!("Unknown input node '{}'", req.node_id));
        };

        binding
            .current_values
            .insert(req.output_id.clone(), req.value.clone());
        binding.updated_at = now_ts();

        interaction.next_event_seq += 1;
        let event = InputEvent {
            seq: interaction.next_event_seq,
            node_id: req.node_id,
            output_id: req.output_id,
            value: req.value,
            timestamp: now_ts(),
        };

        save_state(&state.home, &run_id, &interaction)?;
        append_input_event(&state.home, &run_id, event)?;

        Ok(InteractionSnapshot {
            displays: interaction.displays,
            inputs: interaction.inputs,
        })
    })();

    match result {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(e) => err(e).into_response(),
    }
}

pub async fn claim_input_events(
    State(state): State<AppState>,
    AxumPath((run_id, node_id)): AxumPath<(String, String)>,
    Query(params): Query<ClaimEventsParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);
    match claim_events(&state.home, &run_id, &node_id, since) {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => err(e).into_response(),
    }
}

pub async fn serve_artifact_file(
    State(state): State<AppState>,
    AxumPath((run_id, relative_path)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let relative = match normalize_relative_path(&relative_path) {
        Ok(path) => path,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    };

    let full_path = dm_core::runs::run_out_dir(&state.home, &run_id).join(&relative);
    if !full_path.exists() {
        return (StatusCode::NOT_FOUND, "Artifact file not found").into_response();
    }

    match tokio::fs::read(&full_path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
            let mut resp = bytes.into_response();
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref()).unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            resp
        }
        Err(e) => err(e).into_response(),
    }
}

fn claim_events(home: &Path, run_id: &str, node_id: &str, since: i64) -> anyhow::Result<ClaimEventsResponse> {
    let input_state = load_input_state(home, run_id)?;
    let events: Vec<InputEvent> = input_state
        .events
        .into_iter()
        .filter(|event| event.node_id == node_id && event.seq > since)
        .collect();
    let next_seq = events.last().map(|event| event.seq).unwrap_or(since);
    Ok(ClaimEventsResponse { events, next_seq })
}

fn interaction_dir(home: &Path, run_id: &str) -> PathBuf {
    dm_core::runs::run_dir(home, run_id).join("interaction")
}

fn interaction_state_path(home: &Path, run_id: &str) -> PathBuf {
    interaction_dir(home, run_id).join("state.json")
}

fn interaction_input_path(home: &Path, run_id: &str) -> PathBuf {
    interaction_dir(home, run_id).join("input-events.json")
}

fn ensure_run_exists(home: &Path, run_id: &str) -> anyhow::Result<()> {
    dm_core::runs::load_run(home, run_id).map(|_| ())
}

fn load_state(home: &Path, run_id: &str) -> anyhow::Result<InteractionState> {
    ensure_run_exists(home, run_id)?;
    let path = interaction_state_path(home, run_id);
    if !path.exists() {
        return Ok(InteractionState::default());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_state(home: &Path, run_id: &str, state: &InteractionState) -> anyhow::Result<()> {
    let dir = interaction_dir(home, run_id);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(interaction_state_path(home, run_id), serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn load_input_state(home: &Path, run_id: &str) -> anyhow::Result<InputState> {
    ensure_run_exists(home, run_id)?;
    let path = interaction_input_path(home, run_id);
    if !path.exists() {
        return Ok(InputState {
            events: Vec::new(),
        });
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_input_state(home: &Path, run_id: &str, state: &InputState) -> anyhow::Result<()> {
    let dir = interaction_dir(home, run_id);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(interaction_input_path(home, run_id), serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn mutate_state<F>(home: &Path, run_id: &str, mutator: F) -> anyhow::Result<InteractionState>
where
    F: FnOnce(&mut InteractionState) -> anyhow::Result<()>,
{
    let mut state = load_state(home, run_id)?;
    mutator(&mut state)?;
    save_state(home, run_id, &state)?;
    Ok(state)
}

fn append_input_event(home: &Path, run_id: &str, event: InputEvent) -> anyhow::Result<()> {
    let mut input_state = load_input_state(home, run_id)?;
    input_state.events.push(event);
    if input_state.events.len() > 1000 {
        let keep_from = input_state.events.len().saturating_sub(1000);
        input_state.events = input_state.events.split_off(keep_from);
    }
    save_input_state(home, run_id, &input_state)
}

fn normalize_relative_path(path: &str) -> anyhow::Result<String> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err(anyhow::anyhow!("Expected path relative to run out dir"));
    }

    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            _ => return Err(anyhow::anyhow!("Invalid relative path")),
        }
    }

    let text = normalized.to_string_lossy().to_string();
    if text.is_empty() {
        return Err(anyhow::anyhow!("Path must not be empty"));
    }
    Ok(text)
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}
