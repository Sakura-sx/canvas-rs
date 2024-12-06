use crate::{
    state::{validate_coords, AppState},
    types::{Pixel, PixelQuery},
};
use axum::{extract::Query, response::IntoResponse, Extension, Json};
use serde_json::json;
use std::sync::Arc;

pub async fn get_pixel(
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
pub async fn update_pixel(
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
