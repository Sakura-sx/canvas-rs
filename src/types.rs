use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

pub type Canvas = Arc<RwLock<Vec<Vec<[u8; 3]>>>>;
pub type UpdateChannel = broadcast::Sender<Vec<Pixel>>;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Pixel {
    pub x: u16,
    pub y: u16,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Deserialize)]
pub struct PixelQuery {
    pub x: u16,
    pub y: u16,
}
