use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::broadcast;

use crate::services::{self, message::MessageService};
use crate::state::MessageNotification;

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
enum BridgeMessage {
    #[serde(rename = "init")]
    Init { run_id: String },
    #[serde(rename = "push")]
    Push {
        from: String,
        tag: String,
        payload: Value,
        #[serde(default)]
        timestamp: Option<i64>,
    },
}

pub async fn bridge_socket_loop(
    home: Arc<PathBuf>,
    messages_tx: broadcast::Sender<MessageNotification>,
    listener: UnixListener,
) {
    eprintln!("[bridge-sock] listening for connections");
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let home = home.clone();
                let messages_tx = messages_tx.clone();
                let mut rx = messages_tx.subscribe();
                let (read_half, mut write_half) = tokio::io::split(stream);
                let mut reader = BufReader::new(read_half);

                // 1. Read init message
                let mut init_line = String::new();
                match reader.read_line(&mut init_line).await {
                    Ok(0) | Err(_) => continue,
                    Ok(_) => {}
                }
                let init: BridgeMessage = match serde_json::from_str(init_line.trim()) {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("[bridge-sock] invalid init: {e}");
                        continue;
                    }
                };
                let run_id = match init {
                    BridgeMessage::Init { run_id } => run_id,
                    _ => {
                        eprintln!("[bridge-sock] expected init, got something else");
                        continue;
                    }
                };
                eprintln!("[bridge-sock] bridge connected for run {run_id}");

                // 2. Main select loop
                let mut line_buf = String::new();
                loop {
                    tokio::select! {
                        // Read push messages from bridge
                        read_result = reader.read_line(&mut line_buf) => {
                            match read_result {
                                Ok(0) => {
                                    eprintln!("[bridge-sock] bridge disconnected for run {run_id}");
                                    break;
                                }
                                Ok(_) => {
                                    let msg: BridgeMessage = match serde_json::from_str(line_buf.trim()) {
                                        Ok(m) => m,
                                        Err(e) => {
                                            eprintln!("[bridge-sock] parse error: {e}");
                                            line_buf.clear();
                                            continue;
                                        }
                                    };
                                    line_buf.clear();
                                    if let BridgeMessage::Push { from, tag, payload, timestamp } = msg {
                                        handle_push(&home, &run_id, &messages_tx, &from, &tag, &payload, timestamp);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[bridge-sock] read error: {e}");
                                    break;
                                }
                            }
                        }
                        // Forward input notifications to bridge
                        event = rx.recv() => {
                            let Ok(event) = event else { break };
                            if event.run_id != run_id || event.tag != "input" {
                                continue;
                            }
                            // Look up full message and forward inline payload
                            if let Some(input_msg) = lookup_input(&home, &run_id, event.seq) {
                                let line = format!("{}\n", json!({
                                    "action": "input",
                                    "to": input_msg.payload.get("to").and_then(Value::as_str).unwrap_or(""),
                                    "value": input_msg.payload.get("value").cloned().unwrap_or(Value::Null),
                                }));
                                if write_half.write_all(line.as_bytes()).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[bridge-sock] accept error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

fn handle_push(
    home: &PathBuf,
    run_id: &str,
    messages_tx: &broadcast::Sender<MessageNotification>,
    from: &str,
    tag: &str,
    payload: &Value,
    timestamp: Option<i64>,
) {
    let result = (|| {
        let normalized = if tag == "input" {
            payload.clone()
        } else {
            payload.clone()
        };
        let service = MessageService::open(home, run_id)?;
        let ts = timestamp.unwrap_or_else(services::now_ts);
        let seq = service.push(from, tag, &normalized, ts)?;
        Ok::<i64, anyhow::Error>(seq)
    })();

    match result {
        Ok(seq) => {
            let _ = messages_tx.send(MessageNotification {
                run_id: run_id.to_string(),
                seq,
                from: from.to_string(),
                tag: tag.to_string(),
            });
        }
        Err(e) => {
            eprintln!("[bridge-sock] push error for {from}/{tag}: {e}");
        }
    }
}

fn lookup_input(home: &PathBuf, run_id: &str, seq: i64) -> Option<services::message::Message> {
    let service = MessageService::open(home, run_id).ok()?;
    let filter = services::message::MessageFilter {
        after_seq: Some(seq - 1),
        before_seq: Some(seq + 1),
        from: None,
        tag: Some(vec!["input".to_string()]),
        target_to: None,
        limit: Some(1),
        desc: Some(false),
    };
    service.list(&filter).ok()?.messages.into_iter().next()
}
