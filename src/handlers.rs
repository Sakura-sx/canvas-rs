use crate::{
    state::{validate_coords, AppState},
    types::{Pixel, PixelQuery, PixelComponentUpdate, DifficultyResponse},
    pow::validate_pow,
    constants::{POW_DIFFICULTY, POW_SALT_MAX_AGE_MS},
};
use axum::{extract::Query, response::IntoResponse, Extension, Json};
use serde_json::json;
use std::sync::Arc;
use axum::http::StatusCode;

pub async fn get_difficulty() -> impl IntoResponse {
    Json(json!({ "difficulty": POW_DIFFICULTY }))
}

async fn update_component(
    component: usize,
    state: Arc<AppState>,
    data: PixelComponentUpdate,
) -> impl IntoResponse {
    if !validate_coords(data.x, data.y) {
        return (StatusCode::BAD_REQUEST, "Invalid coordinates");
    }

    if !validate_pow(data.value, data.x, data.y, data.salt, &data.hash, POW_DIFFICULTY) {
        return (StatusCode::BAD_REQUEST, "Invalid proof of work");
    }

    let mut canvas = state.canvas.write();
    canvas[data.y as usize][data.x as usize][component] = data.value;
    
    let pixel = canvas[data.y as usize][data.x as usize];
    let _ = state.update_tx.send(vec![Pixel {
        x: data.x,
        y: data.y,
        r: pixel[0],
        g: pixel[1],
        b: pixel[2],
    }]);

    StatusCode::OK
}

pub async fn update_red(
    Extension(state): Extension<Arc<AppState>>,
    Json(data): Json<PixelComponentUpdate>,
) -> impl IntoResponse {
    update_component(0, state, data).await
}

pub async fn update_green(
    Extension(state): Extension<Arc<AppState>>,
    Json(data): Json<PixelComponentUpdate>,
) -> impl IntoResponse {
    update_component(1, state, data).await
}

pub async fn update_blue(
    Extension(state): Extension<Arc<AppState>>,
    Json(data): Json<PixelComponentUpdate>,
) -> impl IntoResponse {
    update_component(2, state, data).await
}