use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::services;
use crate::services::input::{InputEventWrite, InputRegistration, InputService};
use crate::services::stream::{MessageFilter, StreamPush, StreamService};
use crate::state::{AppState, InputEventNotification, InteractionNotification};

use utoipa::ToSchema;

// ─── Request / Query types ───

#[derive(Debug, Deserialize, ToSchema)]
pub struct StreamPushRequest {
    pub node_id: String,
    pub label: String,
    pub kind: Option<String>,
    pub file: Option<String>,
    pub content: Option<serde_json::Value>,
    pub render: String,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterInputRequest {
    pub node_id: String,
    pub label: Option<String>,
    pub widgets: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EmitInputEventRequest {
    pub node_id: String,
    pub output_id: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ClaimEventsParams {
    pub since: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListStreamMessagesParams {
    pub after_seq: Option<i64>,
    pub before_seq: Option<i64>,
    pub source_id: Option<String>,
    pub limit: Option<usize>,
    pub desc: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InputWsParams {
    pub since: Option<i64>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ClaimEventsResponse {
    pub events: Vec<services::input::InputEvent>,
    pub next_seq: i64,
}

// ─── Handlers ───

#[utoipa::path(
    get,
    path = "/api/runs/{id}/interaction",
    params(("id" = String, Path, description = "Run ID")),
    responses((status = 200, description = "Interaction snapshot"))
)]
pub async fn get_interaction(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let streams = StreamService::open(&state.home, &run_id)?.sources()?;
        let inputs = InputService::open(&state.home, &run_id)?.bindings()?;
        Ok(serde_json::json!({ "streams": streams, "inputs": inputs }))
    })();

    match result {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(e) => err(e).into_response(),
    }
}

pub async fn interaction_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_interaction_ws(socket, state, run_id))
}

pub async fn input_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AxumPath((run_id, node_id)): AxumPath<(String, String)>,
    Query(params): Query<InputWsParams>,
) -> Response {
    ws.on_upgrade(move |socket| {
        handle_input_ws(socket, state, run_id, node_id, params.since.unwrap_or(0))
    })
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/interaction/stream",
    params(("id" = String, Path, description = "Run ID")),
    request_body = StreamPushRequest,
    responses((status = 200, description = "Pushed stream message seq"))
)]
pub async fn post_stream(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<StreamPushRequest>,
) -> impl IntoResponse {
    let source_id = req.node_id.clone();
    let result = (|| {
        let service = StreamService::open(&state.home, &run_id)?;
        let kind = req.kind.unwrap_or_else(|| {
            if req.file.is_some() {
                "file".to_string()
            } else {
                "inline".to_string()
            }
        });
        let file = match req.file.as_deref() {
            Some(file) => Some(services::normalize_relative_path(file)?),
            None => None,
        };
        service.push(StreamPush {
            node_id: req.node_id,
            label: req.label,
            kind,
            file,
            content: req.content,
            render: req.render,
            timestamp: req.timestamp.unwrap_or_else(services::now_ts),
        })
    })();

    match result {
        Ok(seq) => {
            let _ = state.interaction_events.send(InteractionNotification {
                event: "stream.pushed".to_string(),
                run_id,
                source_id: Some(source_id),
                seq: Some(seq),
            });
            Json(serde_json::json!({ "seq": seq })).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}/interaction/stream/messages",
    params(
        ("id" = String, Path, description = "Run ID"),
        ("after_seq" = Option<i64>, Query, description = "Fetch messages after this seq"),
        ("before_seq" = Option<i64>, Query, description = "Fetch messages before this seq"),
        ("source_id" = Option<String>, Query, description = "Filter by source node ID"),
        ("limit" = Option<usize>, Query, description = "Max messages to return"),
        ("desc" = Option<bool>, Query, description = "Reverse order")
    ),
    responses((status = 200, description = "Stream messages with pagination"))
)]
pub async fn list_stream_messages(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Query(params): Query<ListStreamMessagesParams>,
) -> impl IntoResponse {
    let result = (|| {
        let service = StreamService::open(&state.home, &run_id)?;
        service.list(&MessageFilter {
            after_seq: params.after_seq,
            before_seq: params.before_seq,
            source_id: params.source_id,
            limit: params.limit,
            desc: params.desc,
        })
    })();

    match result {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/interaction/input/register",
    params(("id" = String, Path, description = "Run ID")),
    request_body = RegisterInputRequest,
    responses((status = 200, description = "Input registered"))
)]
pub async fn register_input(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<RegisterInputRequest>,
) -> impl IntoResponse {
    let source_id = req.node_id.clone();
    let result = (|| {
        let service = InputService::open(&state.home, &run_id)?;
        service.register(InputRegistration {
            node_id: req.node_id.clone(),
            label: req.label.unwrap_or(req.node_id),
            widgets: req.widgets,
        })
    })();

    match result {
        Ok(()) => {
            let _ = state.interaction_events.send(InteractionNotification {
                event: "input.registered".to_string(),
                run_id,
                source_id: Some(source_id),
                seq: None,
            });
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/interaction/input/events",
    params(("id" = String, Path, description = "Run ID")),
    request_body = EmitInputEventRequest,
    responses((status = 200, description = "Emitted input event"))
)]
pub async fn emit_input_event(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<EmitInputEventRequest>,
) -> impl IntoResponse {
    let result = (|| {
        let service = InputService::open(&state.home, &run_id)?;
        service.emit(InputEventWrite {
            node_id: req.node_id,
            output_id: req.output_id,
            value: req.value,
            timestamp: services::now_ts(),
        })
    })();

    match result {
        Ok(event) => {
            let _ = state.interaction_events.send(InteractionNotification {
                event: "input.event.created".to_string(),
                run_id: run_id.clone(),
                source_id: Some(event.node_id.clone()),
                seq: Some(event.seq),
            });
            let _ = state.input_events.send(InputEventNotification {
                run_id,
                event: event.clone(),
            });
            Json(event).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}/interaction/input/claim/{node_id}",
    params(
        ("id" = String, Path, description = "Run ID"),
        ("node_id" = String, Path, description = "Input node ID"),
        ("since" = Option<i64>, Query, description = "Claim events after this seq")
    ),
    responses((status = 200, description = "Claimed input events"))
)]
pub async fn claim_input_events(
    State(state): State<AppState>,
    AxumPath((run_id, node_id)): AxumPath<(String, String)>,
    Query(params): Query<ClaimEventsParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);
    let result = (|| -> anyhow::Result<ClaimEventsResponse> {
        let service = InputService::open(&state.home, &run_id)?;
        let events = service.claim(&node_id, since)?;
        let next_seq = events.last().map(|event| event.seq).unwrap_or(since);
        Ok(ClaimEventsResponse { events, next_seq })
    })();

    match result {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => err(e).into_response(),
    }
}

// ─── WebSocket handlers ───

async fn handle_interaction_ws(mut socket: WebSocket, state: AppState, run_id: String) {
    let mut rx = state.interaction_events.subscribe();

    loop {
        tokio::select! {
            recv = socket.recv() => {
                match recv {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                    Some(Ok(_)) => {}
                }
            }
            event = rx.recv() => {
                let Ok(event) = event else {
                    return;
                };
                if event.run_id != run_id {
                    continue;
                }
                let payload = match serde_json::to_string(&event) {
                    Ok(payload) => payload,
                    Err(_) => continue,
                };
                if socket.send(Message::Text(payload.into())).await.is_err() {
                    return;
                }
            }
        }
    }
}

async fn handle_input_ws(
    mut socket: WebSocket,
    state: AppState,
    run_id: String,
    node_id: String,
    since: i64,
) {
    let mut rx = state.input_events.subscribe();
    let service = match InputService::open(&state.home, &run_id) {
        Ok(service) => service,
        Err(err) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "error",
                        "message": err.to_string(),
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    };

    let mut last_seq = since;
    let replay = match service.claim(&node_id, since) {
        Ok(events) => events,
        Err(err) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "error",
                        "message": err.to_string(),
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    };

    for event in replay {
        last_seq = event.seq;
        if send_input_event(&mut socket, &event).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            recv = socket.recv() => {
                match recv {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                    Some(Ok(_)) => {}
                }
            }
            message = rx.recv() => {
                let Ok(message) = message else {
                    return;
                };
                if message.run_id != run_id || message.event.node_id != node_id || message.event.seq <= last_seq {
                    continue;
                }
                last_seq = message.event.seq;
                if send_input_event(&mut socket, &message.event).await.is_err() {
                    return;
                }
            }
        }
    }
}

async fn send_input_event(
    socket: &mut WebSocket,
    event: &services::input::InputEvent,
) -> Result<(), ()> {
    let payload = serde_json::json!({
        "type": "input.event",
        "event": event,
    })
    .to_string();
    socket
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}

// ─── Artifact file serving ───

#[utoipa::path(
    get,
    path = "/api/runs/{id}/artifacts/{path}",
    params(
        ("id" = String, Path, description = "Run ID"),
        ("path" = String, Path, description = "Relative artifact path")
    ),
    responses(
        (status = 200, description = "Artifact file content"),
        (status = 404, description = "File not found")
    )
)]
pub async fn serve_artifact_file(
    State(state): State<AppState>,
    AxumPath((run_id, relative_path)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let relative = match services::normalize_relative_path(&relative_path) {
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
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            resp
        }
        Err(e) => err(e).into_response(),
    }
}
