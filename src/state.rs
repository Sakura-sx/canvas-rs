use crate::constants::CANVAS_SIZE;
use crate::persistance;
use crate::types::{Canvas, UpdateChannel};

pub struct AppState {
    pub canvas: Canvas,
    pub update_tx: UpdateChannel,
}

pub fn init_canvas() -> Canvas {
    persistance::load_canvas()
}

#[inline(always)]
pub fn validate_coords(x: u16, y: u16) -> bool {
    x < CANVAS_SIZE as u16 && y < CANVAS_SIZE as u16
}
