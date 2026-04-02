use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::interaction_service::{
    self, DisplayMessagesResponse, DisplayUpdate, InputEventWrite, InputRegistration,
    InteractionMessageService,
};
use crate::{AppState, InputEventNotification, InteractionNotification};

#[derive(Debug, Deserialize)]
pub struct DisplayUpdateRequest {
    pub node_id: String,
    pub label: String,
    pub kind: Option<String>,
    pub file: Option<String>,
    pub content: Option<serde_json::Value>,
    pub render: String,
    pub tail: Option<bool>,
    pub max_lines: Option<usize>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterInputRequest {
    pub node_id: String,
    pub label: Option<String>,
    pub widgets: std::collections::BTreeMap<String, serde_json::Value>,
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

#[derive(Debug, Deserialize)]
pub struct ListMessagesParams {
    pub after_seq: Option<i64>,
    pub before_seq: Option<i64>,
    pub source_id: Option<String>,
    pub limit: Option<usize>,
    pub desc: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct InputWsParams {
    pub since: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct ClaimEventsResponse {
    pub events: Vec<interaction_service::InputEvent>,
    pub next_seq: i64,
}

pub async fn get_interaction(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    match InteractionMessageService::open(&state.home, &run_id).and_then(|svc| svc.snapshot()) {
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

pub async fn post_display(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<DisplayUpdateRequest>,
) -> impl IntoResponse {
    let source_id = req.node_id.clone();
    let result = (|| {
        let service = InteractionMessageService::open(&state.home, &run_id)?;
        let kind = req
            .kind
            .unwrap_or_else(|| {
                if req.file.is_some() {
                    "file".to_string()
                } else {
                    "inline".to_string()
                }
            });
        let file = match req.file.as_deref() {
            Some(file) => Some(interaction_service::normalize_relative_path(file)?),
            None => None,
        };
        service.upsert_display(DisplayUpdate {
            node_id: req.node_id,
            label: req.label,
            kind,
            file,
            content: req.content,
            render: req.render,
            tail: req.tail.unwrap_or(true),
            max_lines: req.max_lines.unwrap_or(500),
            timestamp: req.timestamp.unwrap_or_else(interaction_service::now_ts),
        })
    })();

    match result {
        Ok(result) => {
            let _ = state.interaction_events.send(InteractionNotification {
                event: "display.updated".to_string(),
                run_id,
                source_id: Some(source_id),
                seq: Some(result.seq),
            });
            Json(result.snapshot).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

pub async fn list_display_messages(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Query(params): Query<ListMessagesParams>,
) -> impl IntoResponse {
    let result = (|| -> anyhow::Result<DisplayMessagesResponse> {
        let service = InteractionMessageService::open(&state.home, &run_id)?;
        service.list_display_messages(&interaction_service::MessageFilter {
            after_seq: params.after_seq,
            before_seq: params.before_seq,
            source_id: params.source_id,
            limit: params.limit,
            desc: params.desc,
        })
    })();

    match result {
        Ok(messages) => Json(messages).into_response(),
        Err(e) => err(e).into_response(),
    }
}

pub async fn register_input(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<RegisterInputRequest>,
) -> impl IntoResponse {
    let source_id = req.node_id.clone();
    let result = (|| {
        let service = InteractionMessageService::open(&state.home, &run_id)?;
        service.register_input(InputRegistration {
            node_id: req.node_id.clone(),
            label: req.label.unwrap_or(req.node_id),
            widgets: req.widgets,
        })
    })();

    match result {
        Ok(snapshot) => {
            let _ = state.interaction_events.send(InteractionNotification {
                event: "input.binding.updated".to_string(),
                run_id,
                source_id: Some(source_id),
                seq: None,
            });
            Json(snapshot).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

pub async fn emit_input_event(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<EmitInputEventRequest>,
) -> impl IntoResponse {
    let source_id = req.node_id.clone();
    let result = (|| {
        let service = InteractionMessageService::open(&state.home, &run_id)?;
        service.emit_input_event(InputEventWrite {
            node_id: req.node_id,
            output_id: req.output_id,
            value: req.value,
            timestamp: interaction_service::now_ts(),
        })
    })();

    match result {
        Ok(result) => {
            let _ = state.interaction_events.send(InteractionNotification {
                event: "input.event.created".to_string(),
                run_id: run_id.clone(),
                source_id: Some(source_id),
                seq: Some(result.event.seq),
            });
            let _ = state.input_events.send(InputEventNotification {
                run_id,
                event: result.event,
            });
            Json(result.snapshot).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

pub async fn claim_input_events(
    State(state): State<AppState>,
    AxumPath((run_id, node_id)): AxumPath<(String, String)>,
    Query(params): Query<ClaimEventsParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);
    let result = (|| -> anyhow::Result<ClaimEventsResponse> {
        let service = InteractionMessageService::open(&state.home, &run_id)?;
        let events = service.claim_input_events(&node_id, since)?;
        let next_seq = events.last().map(|event| event.seq).unwrap_or(since);
        Ok(ClaimEventsResponse { events, next_seq })
    })();

    match result {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => err(e).into_response(),
    }
}

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
    let service = match InteractionMessageService::open(&state.home, &run_id) {
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
    let replay = match service.claim_input_events(&node_id, since) {
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
    event: &interaction_service::InputEvent,
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

pub async fn serve_artifact_file(
    State(state): State<AppState>,
    AxumPath((run_id, relative_path)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let relative = match interaction_service::normalize_relative_path(&relative_path) {
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
