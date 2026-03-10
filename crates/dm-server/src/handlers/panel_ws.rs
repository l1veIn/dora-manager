use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::Response;
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
pub struct WsQuery {
    pub since: Option<i64>,
}

pub async fn panel_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Query(params): Query<WsQuery>,
) -> Response {
    let since = params.since.unwrap_or(0);
    ws.on_upgrade(move |socket| handle_panel_ws(socket, state, run_id, since))
}

async fn handle_panel_ws(mut socket: WebSocket, state: AppState, run_id: String, mut since: i64) {
    let store = match dm_core::runs::panel::PanelStore::open(&state.home, &run_id) {
        Ok(s) => s,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({"type": "error", "message": e.to_string()}).to_string().into(),
                ))
                .await;
            return;
        }
    };

    let mut ping_counter: u32 = 0;

    loop {
        // Check for incoming messages (close frame, etc.) without blocking
        match tokio::time::timeout(std::time::Duration::from_millis(100), socket.recv()).await {
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => return,
            Ok(Some(Err(_))) => return,
            // Ignore other incoming messages (text commands from client, etc.)
            Ok(Some(Ok(_))) => {}
            // Timeout = no incoming message, proceed to poll
            Err(_) => {}
        }

        // Poll for new assets
        let filter = dm_core::runs::panel::AssetFilter {
            since_seq: Some(since),
            ..Default::default()
        };

        match store.query_assets(&filter) {
            Ok(result) if !result.assets.is_empty() => {
                if let Some(last) = result.assets.last() {
                    since = last.seq;
                }
                let payload = serde_json::json!({
                    "type": "assets",
                    "data": result.assets,
                });
                if socket
                    .send(Message::Text(payload.to_string().into()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Err(_) => return,
            _ => {}
        }

        // Heartbeat every ~10s (100 iterations * 100ms)
        ping_counter += 1;
        if ping_counter >= 100 {
            ping_counter = 0;
            if socket
                .send(Message::Text(
                    serde_json::json!({"type": "ping"}).to_string().into(),
                ))
                .await
                .is_err()
            {
                return;
            }
        }
    }
}

