use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::handlers::err;
use crate::services;
use crate::services::media::{MediaBackendStatus, MediaStatus};
use crate::services::message::{MessageFilter, MessageService, StreamDescriptor, StreamViewer};
use crate::state::{AppState, MessageNotification};

use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PushMessageRequest {
    pub from: String,
    pub tag: String,
    pub payload: serde_json::Value,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct ListMessagesParams {
    pub after_seq: Option<i64>,
    pub before_seq: Option<i64>,
    #[serde(rename = "from")]
    pub from_filter: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<usize>,
    pub desc: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NodeWsParams {
    pub since: Option<i64>,
}

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
        let (streams, inputs) =
            MessageService::open(&state.home, &run_id)?.interaction_summary()?;
        Ok(serde_json::json!({ "streams": streams, "inputs": inputs }))
    })();

    match result {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/messages",
    params(("id" = String, Path, description = "Run ID")),
    request_body = PushMessageRequest,
    responses((status = 200, description = "Pushed message seq"))
)]
pub async fn push_message(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Json(req): Json<PushMessageRequest>,
) -> impl IntoResponse {
    let result = (|| {
        let payload = normalize_payload(&req.tag, req.payload)?;
        let service = MessageService::open(&state.home, &run_id)?;
        service.push(
            &req.from,
            &req.tag,
            &payload,
            req.timestamp.unwrap_or_else(services::now_ts),
        )
    })();

    match result {
        Ok(seq) => {
            let _ = state.messages.send(MessageNotification {
                run_id,
                seq,
                from: req.from,
                tag: req.tag,
            });
            Json(serde_json::json!({ "seq": seq })).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}/messages",
    params(
        ("id" = String, Path, description = "Run ID"),
        ("after_seq" = Option<i64>, Query, description = "Fetch messages after this seq"),
        ("before_seq" = Option<i64>, Query, description = "Fetch messages before this seq"),
        ("from" = Option<String>, Query, description = "Comma-separated source node IDs"),
        ("tag" = Option<String>, Query, description = "Comma-separated tags"),
        ("limit" = Option<usize>, Query, description = "Max messages to return"),
        ("desc" = Option<bool>, Query, description = "Reverse order")
    ),
    responses((status = 200, description = "Messages with pagination"))
)]
pub async fn list_messages(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Query(params): Query<ListMessagesParams>,
) -> impl IntoResponse {
    let result = (|| {
        let service = MessageService::open(&state.home, &run_id)?;
        service.list(&MessageFilter {
            after_seq: params.after_seq,
            before_seq: params.before_seq,
            from: split_csv(params.from_filter),
            tag: split_csv(params.tag),
            target_to: None,
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
    get,
    path = "/api/runs/{id}/messages/snapshots",
    params(("id" = String, Path, description = "Run ID")),
    responses((status = 200, description = "Latest snapshots"))
)]
pub async fn get_snapshots(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    let result = (|| MessageService::open(&state.home, &run_id)?.snapshots())();

    match result {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}/streams",
    params(("id" = String, Path, description = "Run ID")),
    responses((status = 200, description = "Resolved stream descriptors"))
)]
pub async fn list_streams(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> impl IntoResponse {
    let media_status = state.media.status().await;
    let result = (|| {
        let service = MessageService::open(&state.home, &run_id)?;
        let snapshots = service.stream_snapshots()?;
        snapshots
            .into_iter()
            .map(|snapshot| stream_descriptor_from_snapshot(&state, &media_status, snapshot))
            .collect::<anyhow::Result<Vec<_>>>()
    })();

    match result {
        Ok(streams) => Json(serde_json::json!({ "streams": streams })).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}/streams/{stream_id}",
    params(
        ("id" = String, Path, description = "Run ID"),
        ("stream_id" = String, Path, description = "Stream ID")
    ),
    responses(
        (status = 200, description = "Single stream descriptor"),
        (status = 404, description = "Stream not found")
    )
)]
pub async fn get_stream(
    State(state): State<AppState>,
    AxumPath((run_id, stream_id)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let media_status = state.media.status().await;
    let result = (|| {
        let service = MessageService::open(&state.home, &run_id)?;
        let snapshot = service
            .stream_snapshots()?
            .into_iter()
            .find(|snapshot| {
                snapshot
                    .payload
                    .get("stream_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(stream_id.as_str())
            })
            .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
        stream_descriptor_from_snapshot(&state, &media_status, snapshot)
    })();

    match result {
        Ok(stream) => Json(stream).into_response(),
        Err(e) if e.to_string() == "Stream not found" => {
            (StatusCode::NOT_FOUND, e.to_string()).into_response()
        }
        Err(e) => err(e).into_response(),
    }
}

pub async fn messages_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_messages_ws(socket, state, run_id))
}

pub async fn node_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AxumPath((run_id, node_id)): AxumPath<(String, String)>,
    Query(params): Query<NodeWsParams>,
) -> Response {
    ws.on_upgrade(move |socket| {
        handle_node_ws(socket, state, run_id, node_id, params.since.unwrap_or(0))
    })
}

async fn handle_messages_ws(mut socket: WebSocket, state: AppState, run_id: String) {
    let mut rx = state.messages.subscribe();

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

async fn handle_node_ws(
    mut socket: WebSocket,
    state: AppState,
    run_id: String,
    node_id: String,
    since: i64,
) {
    let service = match MessageService::open(&state.home, &run_id) {
        Ok(service) => service,
        Err(err) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({ "type": "error", "message": err.to_string() })
                        .to_string()
                        .into(),
                ))
                .await;
            return;
        }
    };

    let replay = match service.list(&MessageFilter {
        after_seq: Some(since),
        target_to: Some(node_id.clone()),
        limit: None,
        ..Default::default()
    }) {
        Ok(resp) => resp.messages,
        Err(err) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({ "type": "error", "message": err.to_string() })
                        .to_string()
                        .into(),
                ))
                .await;
            return;
        }
    };

    let mut last_seq = since;
    for event in replay {
        last_seq = event.seq;
        if send_node_message(&mut socket, &event).await.is_err() {
            return;
        }
    }

    let mut rx = state.messages.subscribe();
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
                if message.run_id != run_id || message.seq <= last_seq {
                    continue;
                }
                if message.from != "web" && message.tag != "input" {
                    continue;
                }
                let resp = match service.list(&MessageFilter {
                    after_seq: Some(last_seq),
                    target_to: Some(node_id.clone()),
                    limit: None,
                    ..Default::default()
                }) {
                    Ok(resp) => resp,
                    Err(_) => continue,
                };
                for event in resp.messages {
                    if event.seq <= last_seq {
                        continue;
                    }
                    last_seq = event.seq;
                    if send_node_message(&mut socket, &event).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

async fn send_node_message(
    socket: &mut WebSocket,
    event: &crate::services::message::Message,
) -> Result<(), ()> {
    socket
        .send(Message::Text(
            serde_json::to_string(event).map_err(|_| ())?.into(),
        ))
        .await
        .map_err(|_| ())
}

fn split_csv(value: Option<String>) -> Option<Vec<String>> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .filter(|items: &Vec<String>| !items.is_empty())
}

fn normalize_payload(tag: &str, payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    if tag == "input" {
        return Ok(payload);
    }

    if tag == "stream" {
        return normalize_stream_payload(payload);
    }

    if let Some(file) = payload.get("file").and_then(serde_json::Value::as_str) {
        let mut object = payload
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Payload must be an object"))?;
        object.insert(
            "file".to_string(),
            serde_json::Value::String(services::normalize_relative_path(file)?),
        );
        return Ok(serde_json::Value::Object(object));
    }

    Ok(payload)
}

fn normalize_stream_payload(payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let mut object = payload
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Stream payload must be an object"))?;

    let path = object
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Stream payload requires 'path'"))?
        .to_string();
    let stream_id = object
        .get("stream_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Stream payload requires 'stream_id'"))?
        .to_string();
    let kind = object
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Stream payload requires 'kind'"))?
        .to_string();

    object.insert(
        "path".to_string(),
        serde_json::Value::String(services::normalize_relative_path(&path)?),
    );
    object.insert(
        "stream_id".to_string(),
        serde_json::Value::String(stream_id),
    );
    object.insert("kind".to_string(), serde_json::Value::String(kind));
    if !object.contains_key("live") {
        object.insert("live".to_string(), serde_json::Value::Bool(true));
    }

    Ok(serde_json::Value::Object(object))
}

fn stream_descriptor_from_snapshot(
    state: &AppState,
    media_status: &MediaStatus,
    snapshot: crate::services::message::MessageSnapshot,
) -> anyhow::Result<StreamDescriptor> {
    let payload = snapshot.payload;
    let stream_id = payload
        .get("stream_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid stream snapshot: missing stream_id"))?;
    let path = payload
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid stream snapshot: missing path"))?;
    let kind = payload
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("video")
        .to_string();
    let label = payload
        .get("label")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&snapshot.node_id)
        .to_string();

    let viewer = match media_status.status {
        MediaBackendStatus::Ready => Some(StreamViewer {
            preferred: "webrtc".to_string(),
            webrtc_url: Some(format!("{}/{}", state.media.webrtc_base_url(), path)),
            hls_url: Some(format!(
                "{}/{}/index.m3u8",
                state.media.hls_base_url(),
                path
            )),
        }),
        _ => None,
    };

    Ok(StreamDescriptor {
        stream_id: stream_id.to_string(),
        from: snapshot.node_id.clone(),
        kind,
        label,
        path: path.to_string(),
        live: payload
            .get("live")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        codec: payload
            .get("codec")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        width: payload
            .get("width")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as u32),
        height: payload
            .get("height")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as u32),
        fps: payload
            .get("fps")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as u32),
        seq: snapshot.seq,
        updated_at: snapshot.updated_at,
        viewer,
    })
}

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
