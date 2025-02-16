use tokio::time::Duration;

pub const CANVAS_SIZE: usize = 1024;
pub const CHUNK_SIZE: usize = 5000;
pub const UPDATE_INTERVAL: Duration = Duration::from_millis(16);
pub const BROADCAST_CHANNEL_SIZE: usize = 16384;
pub const POW_DIFFICULTY: u64 = 10_000;
pub const POW_SALT_MAX_AGE_MS: u64 = 5000;