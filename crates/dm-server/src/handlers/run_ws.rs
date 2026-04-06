use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, State};
use axum::response::Response;
use notify::{EventKind, RecursiveMode, Watcher};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::mpsc;

use crate::state::AppState;
use dm_core::runs::{run_logs_dir, run_out_dir};

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum WsMessage {
    Ping,
    Metrics {
        data: Vec<dm_core::runs::NodeMetrics>,
    },
    #[serde(rename_all = "camelCase")]
    Logs {
        node_id: String,
        lines: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Io {
        node_id: String,
        lines: Vec<String>,
    },
    Status {
        status: String,
    },
}

pub async fn run_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_run_ws(socket, state, run_id))
}

fn resolve_logs_dir(home: &Path, run_id: &str) -> (PathBuf, bool) {
    if let Ok(run) = dm_core::runs::load_run(home, run_id) {
        if let Some(ref uuid) = run.dora_uuid {
            let live_dir = run_out_dir(home, run_id).join(uuid);
            if live_dir.exists() {
                return (live_dir, true);
            }
        }
    }
    (run_logs_dir(home, run_id), false)
}

fn node_id_from_filename(filename: &str, is_live: bool) -> Option<String> {
    if is_live {
        filename
            .strip_prefix("log_")
            .and_then(|s| s.strip_suffix(".txt"))
            .map(|s| s.to_string())
    } else {
        filename.strip_suffix(".log").map(|s| s.to_string())
    }
}

async fn handle_run_ws(mut socket: WebSocket, state: AppState, run_id: String) {
    let mut log_offsets: HashMap<PathBuf, u64> = HashMap::new();
    let (tx, mut rx) = mpsc::channel::<PathBuf>(1024);

    let watcher_tx = tx.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res {
            if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                for path in event.paths {
                    let _ = watcher_tx.blocking_send(path);
                }
            }
        }
    })
    .unwrap();

    let (mut current_logs_dir, mut is_live) = resolve_logs_dir(&state.home, &run_id);
    if current_logs_dir.exists() {
        let _ = watcher.watch(&current_logs_dir, RecursiveMode::NonRecursive);
    }

    let mut metrics_interval = tokio::time::interval(Duration::from_secs(1));
    let mut ping_interval = tokio::time::interval(Duration::from_secs(10));

    // Force immediate first read for existing logs
    if current_logs_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&current_logs_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.path().is_file() {
                    let _ = tx.send(entry.path()).await;
                }
            }
        }
    }

    loop {
        tokio::select! {
            Some(path) = rx.recv() => {
                let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Some(node_id) = node_id_from_filename(&filename, is_live) {
                    if !read_and_push_logs(&mut socket, &path, node_id, &mut log_offsets).await {
                        return;
                    }
                }
            }
            _ = metrics_interval.tick() => {
                let (new_logs_dir, new_is_live) = resolve_logs_dir(&state.home, &run_id);
                if new_logs_dir != current_logs_dir {
                    let _ = watcher.unwatch(&current_logs_dir);
                    if new_logs_dir.exists() {
                        let _ = watcher.watch(&new_logs_dir, RecursiveMode::NonRecursive);
                        current_logs_dir = new_logs_dir;
                        is_live = new_is_live;
                        // Scan existing files in the new directory
                        if let Ok(mut entries) = tokio::fs::read_dir(&current_logs_dir).await {
                            while let Ok(Some(entry)) = entries.next_entry().await {
                                if entry.path().is_file() {
                                    let _ = tx.send(entry.path()).await;
                                }
                            }
                        }
                    }
                }

                if !push_metrics_and_status(&mut socket, &state, &run_id).await {
                    return;
                }
            }
            _ = ping_interval.tick() => {
                if send_msg(&mut socket, &WsMessage::Ping).await.is_err() { return; }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Err(_)) => return,
                    _ => {}
                }
            }
        }
    }
}

async fn read_and_push_logs(
    socket: &mut WebSocket,
    path: &PathBuf,
    node_id: String,
    log_offsets: &mut HashMap<PathBuf, u64>,
) -> bool {
    if let Ok(metadata) = tokio::fs::metadata(&path).await {
        let len = metadata.len();
        let offset = *log_offsets.get(path).unwrap_or(&0);

        if len > offset {
            if let Ok(mut file) = tokio::fs::File::open(&path).await {
                if file.seek(std::io::SeekFrom::Start(offset)).await.is_ok() {
                    let mut buffer = String::new();
                    if file.read_to_string(&mut buffer).await.is_ok() {
                        log_offsets.insert(path.clone(), len);

                        let lines: Vec<&str> = buffer.lines().collect();
                        if !lines.is_empty() {
                            let general_lines: Vec<String> =
                                lines.iter().map(|s| s.to_string()).collect();

                            if !general_lines.is_empty() {
                                let msg = WsMessage::Logs {
                                    node_id: node_id.clone(),
                                    lines: general_lines,
                                };
                                if send_msg(socket, &msg).await.is_err() {
                                    return false;
                                }
                            }

                            let io_lines: Vec<String> = lines
                                .into_iter()
                                .filter(|l| l.contains("[DM-IO]"))
                                .map(|l| l.to_string())
                                .collect();

                            if !io_lines.is_empty() {
                                let msg = WsMessage::Io {
                                    node_id,
                                    lines: io_lines,
                                };
                                if send_msg(socket, &msg).await.is_err() {
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    true
}

async fn push_metrics_and_status(socket: &mut WebSocket, state: &AppState, run_id: &str) -> bool {
    if let Ok(run_detail) = dm_core::runs::get_run(&state.home, run_id) {
        let status_msg = WsMessage::Status {
            status: run_detail.summary.status.clone(),
        };
        if send_msg(socket, &status_msg).await.is_err() {
            return false;
        }

        if "Running" == run_detail.summary.status {
            if let Ok(Some(metrics)) = dm_core::runs::get_run_metrics(&state.home, run_id) {
                let metrics_msg = WsMessage::Metrics {
                    data: metrics.nodes,
                };
                if send_msg(socket, &metrics_msg).await.is_err() {
                    return false;
                }
            }
        }
    }
    true
}

async fn send_msg(socket: &mut WebSocket, msg: &WsMessage) -> Result<(), ()> {
    let payload = serde_json::to_string(msg).unwrap_or_default();
    if socket.send(Message::Text(payload.into())).await.is_err() {
        return Err(());
    }
    Ok(())
}
