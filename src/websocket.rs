use crate::{
    constants::{CANVAS_SIZE, CHUNK_SIZE, UPDATE_INTERVAL, POW_DIFFICULTY, POW_SALT_MAX_AGE_MS},
    state::{validate_coords, AppState},
    types::Pixel,
    pow::validate_pow,
};
use axum::{
    extract::ws::{Message, WebSocket},
    extract::WebSocketUpgrade,
    response::IntoResponse,
    Extension,
};
use std::sync::Arc;
use tokio::time::interval;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WsPixelUpdate {
    component: String,
    x: u16,
    y: u16,
    value: u8,
    salt: u64,
    hash: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_canvas_websocket(socket, state))
}

pub async fn cws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_drawing_websocket(socket, state))
}

async fn handle_drawing_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut update_interval = interval(UPDATE_INTERVAL);
    let mut update_buffer = Vec::new();

    loop {
        tokio::select! {
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    if let Ok(update) = serde_json::from_str::<WsPixelUpdate>(&text) {
                        let component = match update.component.as_str() {
                            "r" => 0,
                            "g" => 1,
                            "b" => 2,
                            _ => {
                                let _ = socket.send(Message::Text("err".to_string())).await;
                                continue;
                            }
                        };

                        if !validate_pow(update.value, update.x, update.y, update.salt, &update.hash, POW_DIFFICULTY) {
                            let _ = socket.send(Message::Text("err".to_string())).await;
                            continue;
                        }

                        update_buffer.push((component, update));
                    }
                }
            }
            _ = update_interval.tick() => {
                if !update_buffer.is_empty() {
                    let mut canvas = state.canvas.write();
                    let mut pixels = Vec::new();
                    
                    for (component, update) in update_buffer.drain(..) {
                        let pixel = &mut canvas[update.y as usize][update.x as usize];
                        pixel[component] = update.value;
                        
                        pixels.push(Pixel {
                            x: update.x,
                            y: update.y,
                            r: pixel[0],
                            g: pixel[1],
                            b: pixel[2],
                        });
                    }
                    
                    let _ = state.update_tx.send(pixels);
                    let _ = socket.send(Message::Text("ok".to_string())).await;
                }
            }
        }
    }
}