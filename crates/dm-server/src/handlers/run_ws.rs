use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::AppState;
use dm_core::runs::run_logs_dir;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum WsMessage {
    Ping,
    Metrics { data: Vec<dm_core::runs::NodeMetrics> },
    Logs { node_id: String, lines: Vec<String> },
    Io { node_id: String, lines: Vec<String> },
    Status { status: String },
    Error { message: String },
}

pub async fn run_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_run_ws(socket, state, run_id))
}

async fn handle_run_ws(mut socket: WebSocket, state: AppState, run_id: String) {
    let logs_dir = run_logs_dir(&state.home, &run_id);
    
    // Keep track of the offset we have read for each log file
    let mut log_offsets: HashMap<PathBuf, u64> = HashMap::new();
    let mut ping_counter: u32 = 0;
    let mut metrics_counter: u32 = 0;

    loop {
        // Check for incoming messages (close frame, etc.) without blocking
        match tokio::time::timeout(Duration::from_millis(100), socket.recv()).await {
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => return,
            Ok(Some(Err(_))) => return,
            Ok(Some(Ok(_))) => {} // Ignore client messages for now
            Err(_) => {} // Timeout, proceed to tick
        }

        // 1. Poll logs and [DM-IO] tracking
        if let Ok(mut entries) = tokio::fs::read_dir(&logs_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file() && path.extension().unwrap_or_default() == "log" {
                    let node_id = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    
                    if let Ok(metadata) = tokio::fs::metadata(&path).await {
                        let len = metadata.len();
                        let offset = *log_offsets.get(&path).unwrap_or(&0);
                        
                        if len > offset {
                            if let Ok(mut file) = tokio::fs::File::open(&path).await {
                                if file.seek(std::io::SeekFrom::Start(offset)).await.is_ok() {
                                    let mut buffer = String::new();
                                    if file.read_to_string(&mut buffer).await.is_ok() {
                                        // Update offset
                                        log_offsets.insert(path.clone(), len);
                                        
                                        // Process new lines
                                        let lines: Vec<&str> = buffer.lines().collect();
                                        if !lines.is_empty() {
                                            // Extract standard logs
                                            let general_lines: Vec<String> = lines.iter()
                                                .map(|s| s.to_string())
                                                .collect();
                                            
                                            if !general_lines.is_empty() {
                                                let msg = WsMessage::Logs {
                                                    node_id: node_id.clone(),
                                                    lines: general_lines,
                                                };
                                                if send_msg(&mut socket, &msg).await.is_err() { return; }
                                            }

                                            // Extract DM-IO logs
                                            let io_lines: Vec<String> = lines.into_iter()
                                                .filter(|l| l.contains("[DM-IO]"))
                                                .map(|l| l.to_string())
                                                .collect();
                                                
                                            if !io_lines.is_empty() {
                                                let msg = WsMessage::Io {
                                                    node_id,
                                                    lines: io_lines,
                                                };
                                                if send_msg(&mut socket, &msg).await.is_err() { return; }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Poll metrics every 1s (10 ticks * 100ms)
        metrics_counter += 1;
        if metrics_counter >= 10 {
            metrics_counter = 0;
            
            // Fast check the run status and exit status
            match dm_core::runs::get_run(&state.home, &run_id) {
                Ok(run_detail) => {
                    let status_msg = WsMessage::Status { status: run_detail.summary.status.clone() };
                    if send_msg(&mut socket, &status_msg).await.is_err() { return; }
                    
                    // If still active, fetch dora metrics
                    if "Running" == run_detail.summary.status {
                        match dm_core::runs::get_run_metrics(&state.home, &run_id) {
                            Ok(Some(mut metrics)) => {
                                // Ignore panel node in metrics array because we only care about real dataflow nodes
                                metrics.nodes.retain(|n| n.id != "dm-panel");
                                let metrics_msg = WsMessage::Metrics { data: metrics.nodes };
                                if send_msg(&mut socket, &metrics_msg).await.is_err() { return; }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {} // Run missing or error
            }
        }

        // 3. Heartbeat every ~10s (100 ticks)
        ping_counter += 1;
        if ping_counter >= 100 {
            ping_counter = 0;
            if send_msg(&mut socket, &WsMessage::Ping).await.is_err() { return; }
        }
    }
}

async fn send_msg(socket: &mut WebSocket, msg: &WsMessage) -> Result<(), ()> {
    let payload = serde_json::to_string(msg).unwrap_or_default();
    if socket.send(Message::Text(payload.into())).await.is_err() {
        return Err(());
    }
    Ok(())
}
