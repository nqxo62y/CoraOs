use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use tokio::time::interval;
use tracing::{error, info};

use crate::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    info!("WebSocket client connected");

    let mut tick = interval(Duration::from_secs(2));

    loop {
        tick.tick().await;

        let metrics = state.monitor.get_metrics();
        let payload = match serde_json::to_string(&metrics) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize metrics: {}", e);
                continue;
            }
        };

        if socket.send(Message::Text(payload)).await.is_err() {
            info!("WebSocket client disconnected");
            break;
        }
    }
}
