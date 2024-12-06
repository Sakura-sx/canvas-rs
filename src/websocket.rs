use crate::{
    constants::{CANVAS_SIZE, CHUNK_SIZE, UPDATE_INTERVAL},
    state::{validate_coords, AppState},
    types::Pixel,
};
use axum::{
    extract::ws::{Message, WebSocket},
    extract::WebSocketUpgrade,
    response::IntoResponse,
    Extension,
};
use std::sync::Arc;
use tokio::time::interval;

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

async fn handle_canvas_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    {
        for chunk_start in (0..CANVAS_SIZE * CANVAS_SIZE).step_by(CHUNK_SIZE) {
            let mut buffer = Vec::with_capacity(3 + CHUNK_SIZE * 7);
            buffer.push(0x01);

            let chunk_size = std::cmp::min(CHUNK_SIZE, CANVAS_SIZE * CANVAS_SIZE - chunk_start);
            buffer.extend_from_slice(&(chunk_size as u16).to_be_bytes());

            {
                let canvas = state.canvas.read();
                for i in 0..chunk_size {
                    let pos = chunk_start + i;
                    let x = (pos % CANVAS_SIZE) as u16;
                    let y = (pos / CANVAS_SIZE) as u16;
                    let pixel = canvas[y as usize][x as usize];

                    buffer.extend_from_slice(&x.to_be_bytes());
                    buffer.extend_from_slice(&y.to_be_bytes());
                    buffer.extend_from_slice(&pixel);
                }
            }

            if socket.send(Message::Binary(buffer)).await.is_err() {
                println!("Failed to send initial canvas chunk");
                return;
            }
        }
        println!("Sent initial canvas state");
    }

    let mut rx = state.update_tx.subscribe();
    let mut interval = interval(UPDATE_INTERVAL);

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(pixels) => {
                        let mut buffer = Vec::with_capacity(pixels.len() * 7 + 3);
                        buffer.push(0x01);
                        buffer.extend_from_slice(&(pixels.len() as u16).to_be_bytes());

                        for pixel in pixels {
                            buffer.extend_from_slice(&pixel.x.to_be_bytes());
                            buffer.extend_from_slice(&pixel.y.to_be_bytes());
                            buffer.extend([pixel.r, pixel.g, pixel.b]);
                        }

                        if socket.send(Message::Binary(buffer)).await.is_err() {
                            println!("Failed to send update");
                            break;
                        }
                    }
                    Err(e) => {
                        println!("Error receiving update: {}", e);
                        break;
                    }
                }
            }
            _ = interval.tick() => {}
        }
    }
}

async fn handle_drawing_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut update_buffer = Vec::with_capacity(1000);
    let mut update_interval = interval(UPDATE_INTERVAL);

    loop {
        tokio::select! {
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    if let Ok(pixel) = serde_json::from_str::<Pixel>(&text) {
                        if validate_coords(pixel.x, pixel.y) {
                            update_buffer.push(pixel);

                            if update_buffer.len() >= 1000 {
                                {
                                    let mut canvas = state.canvas.write();
                                    for p in &update_buffer {
                                        canvas[p.y as usize][p.x as usize] = [p.r, p.g, p.b];
                                    }
                                }

                                let _ = state.update_tx.send(update_buffer.clone());
                                update_buffer.clear();

                                if socket.send(Message::Text("ok".to_string())).await.is_err() {
                                    break;
                                }
                            }
                        } else if socket.send(Message::Text("err".to_string())).await.is_err() {
                            break;
                        }
                    }
                }
            }
            _ = update_interval.tick() => {
                if !update_buffer.is_empty() {
                    {
                        let mut canvas = state.canvas.write();
                        for p in &update_buffer {
                            canvas[p.y as usize][p.x as usize] = [p.r, p.g, p.b];
                        }
                    }

                    let _ = state.update_tx.send(update_buffer.clone());
                    update_buffer.clear();

                    if socket.send(Message::Text("ok".to_string())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}
