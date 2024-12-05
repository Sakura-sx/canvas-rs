use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use parking_lot::RwLock; // Changed from tokio::sync::RwLock
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tower_http::{cors::CorsLayer, services::ServeDir};

// Constants
const CANVAS_SIZE: usize = 512;
const CHUNK_SIZE: usize = 5000;
const UPDATE_INTERVAL: Duration = Duration::from_millis(16); // Increased frequency
const BROADCAST_CHANNEL_SIZE: usize = 16384; // Increased buffer size

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct Pixel {
    x: u16,
    y: u16,
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Deserialize)]
struct PixelQuery {
    x: u16,
    y: u16,
}

type Canvas = Arc<RwLock<Vec<Vec<[u8; 3]>>>>;
type UpdateChannel = broadcast::Sender<Vec<Pixel>>; // Changed to send batches

struct AppState {
    canvas: Canvas,
    update_tx: UpdateChannel,
}

fn init_canvas() -> Canvas {
    Arc::new(RwLock::new(vec![vec![[255; 3]; CANVAS_SIZE]; CANVAS_SIZE]))
}

fn validate_coords(x: u16, y: u16) -> bool {
    x < CANVAS_SIZE as u16 && y < CANVAS_SIZE as u16
}

async fn get_pixel(
    Query(params): Query<PixelQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    if !validate_coords(params.x, params.y) {
        return (axum::http::StatusCode::BAD_REQUEST, "Invalid coordinates").into_response();
    }

    let canvas = state.canvas.read();
    let pixel = canvas[params.y as usize][params.x as usize];

    (
        axum::http::StatusCode::OK,
        Json(json!({
            "r": pixel[0],
            "g": pixel[1],
            "b": pixel[2]
        })),
    )
        .into_response()
}

#[axum::debug_handler]
async fn update_pixel(
    Extension(state): Extension<Arc<AppState>>,
    Json(pixel): Json<Pixel>,
) -> impl IntoResponse {
    if !validate_coords(pixel.x, pixel.y) {
        return (axum::http::StatusCode::BAD_REQUEST, "Invalid coordinates").into_response();
    }

    let mut canvas = state.canvas.write();
    canvas[pixel.y as usize][pixel.x as usize] = [pixel.r, pixel.g, pixel.b];
    let _ = state.update_tx.send(vec![pixel]);

    axum::http::StatusCode::OK.into_response()
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_canvas_websocket(socket, state))
}

async fn cws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_drawing_websocket(socket, state))
}
async fn handle_canvas_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    println!("New canvas WebSocket connection established");

    // Send initial canvas state in chunks
    {
        for chunk_start in (0..CANVAS_SIZE * CANVAS_SIZE).step_by(CHUNK_SIZE) {
            let mut buffer = Vec::with_capacity(3 + CHUNK_SIZE * 7);
            buffer.push(0x01); // Message type

            let chunk_size = std::cmp::min(CHUNK_SIZE, CANVAS_SIZE * CANVAS_SIZE - chunk_start);
            buffer.extend_from_slice(&(chunk_size as u16).to_be_bytes());

            // Scope the lock to ensure it's dropped before the await
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
            } // Lock is dropped here

            if socket.send(Message::Binary(buffer)).await.is_err() {
                println!("Failed to send initial canvas chunk");
                return;
            }
        }
        println!("Sent initial canvas state");
    }

    // Subscribe to updates
    let mut rx = state.update_tx.subscribe();
    let mut interval = tokio::time::interval(UPDATE_INTERVAL);

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
            _ = interval.tick() => {
                // Keep-alive tick
            }
        }
    }
    println!("WebSocket connection closed");
}

async fn handle_drawing_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut update_buffer = Vec::with_capacity(1000);
    let mut update_interval = tokio::time::interval(UPDATE_INTERVAL);

    loop {
        tokio::select! {
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    if let Ok(pixel) = serde_json::from_str::<Pixel>(&text) {
                        if validate_coords(pixel.x, pixel.y) {
                            update_buffer.push(pixel);

                            if update_buffer.len() >= 1000 {
                                // Scope the lock to ensure it's dropped before the await
                                {
                                    let mut canvas = state.canvas.write();
                                    for p in &update_buffer {
                                        canvas[p.y as usize][p.x as usize] = [p.r, p.g, p.b];
                                    }
                                } // canvas lock is dropped here

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
                    // Scope the lock to ensure it's dropped before the await
                    {
                        let mut canvas = state.canvas.write();
                        for p in &update_buffer {
                            canvas[p.y as usize][p.x as usize] = [p.r, p.g, p.b];
                        }
                    } // canvas lock is dropped here

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

#[tokio::main]
async fn main() {
    let canvas = init_canvas();
    let (update_tx, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);

    let state = Arc::new(AppState {
        canvas: canvas.clone(),
        update_tx: update_tx.clone(),
    });
    let serve_dir = ServeDir::new("static");

    let app = Router::new()
        .route("/getPixel", get(get_pixel))
        .route("/updatePixel", post(update_pixel))
        .route("/ws", get(ws_handler))
        .route("/cws", get(cws_handler))
        .nest_service("/", serve_dir.clone())
        .layer(Extension(state))
        .layer(CorsLayer::permissive());

    println!("Server running on http://127.0.0.1:8000");
    axum::Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
