mod constants;
mod handlers;
mod persistance;
mod state;
mod types;
mod websocket;

use crate::{
    constants::BROADCAST_CHANNEL_SIZE,
    handlers::{get_pixel, update_pixel},
    persistance::save_canvas,
    state::{init_canvas, AppState},
    websocket::{cws_handler, ws_handler},
};
use axum::{
    routing::{get, post},
    Extension, Router,
};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
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
        .route("/ws/stream", get(ws_handler))
        .route("/ws/draw", get(cws_handler))
        .nest_service("/", serve_dir.clone())
        .nest_service("/docs", ServeFile::new("static/docs.html"))
        .layer(Extension(state.clone()))
        .layer(CorsLayer::permissive());

    println!("Server running on http://0.0.0.0:3000");

    let server =
        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap()).serve(app.into_make_service());

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("Server error: {}", e);
    }

    println!("Saving canvas state...");
    if let Err(e) = save_canvas(&state.canvas) {
        eprintln!("Error saving canvas state: {}", e);
    }
    println!("Server shutdown complete");
}
