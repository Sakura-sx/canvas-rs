use crate::constants::CANVAS_SIZE;
use parking_lot::RwLock;
use std::fs;
use std::path::Path;
use std::sync::Arc;

const CANVAS_FILE: &str = "canvas_state.bin";

pub fn save_canvas(canvas: &Arc<RwLock<Vec<Vec<[u8; 3]>>>>) -> Result<(), std::io::Error> {
    let canvas_data = canvas.read();
    let mut buffer = Vec::with_capacity(CANVAS_SIZE * CANVAS_SIZE * 3);

    for row in canvas_data.iter() {
        for pixel in row.iter() {
            buffer.extend_from_slice(pixel);
        }
    }

    fs::write(CANVAS_FILE, buffer)
}

pub fn load_canvas() -> Arc<RwLock<Vec<Vec<[u8; 3]>>>> {
    if Path::new(CANVAS_FILE).exists() {
        if let Ok(buffer) = fs::read(CANVAS_FILE) {
            if buffer.len() == CANVAS_SIZE * CANVAS_SIZE * 3 {
                let mut canvas = vec![vec![[255; 3]; CANVAS_SIZE]; CANVAS_SIZE];
                let mut index = 0;

                for (_, row) in canvas.iter_mut().enumerate().take(CANVAS_SIZE) {
                    for (_, pixel) in row.iter_mut().enumerate().take(CANVAS_SIZE) {
                        *pixel = [buffer[index], buffer[index + 1], buffer[index + 2]];
                        index += 3;
                    }
                }

                return Arc::new(RwLock::new(canvas));
            }
        }
    }

    Arc::new(RwLock::new(vec![vec![[255; 3]; CANVAS_SIZE]; CANVAS_SIZE]))
}
