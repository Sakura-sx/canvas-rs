use sha2::{Sha256, Digest};
use num_bigint::BigUint;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn validate_pow(value: u8, x: u16, y: u16, salt: u64, hash: &str, difficulty: u64) -> bool {
    // Validate timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64;
    
    if now.saturating_sub(salt) > POW_SALT_MAX_AGE_MS {
        return false;
    }

    // Generate hash string
    let data = format!("{},{},{},{}", value, x, y, salt);
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let hex_hash = hex::encode(result);

    // Validate hash matches
    if hex_hash != hash {
        return false;
    }

    // Validate difficulty
    let hash_int = BigUint::from_bytes_be(&result);
    let max_hash = BigUint::from_bytes_be(&[0xff; 32]);
    let target = max_hash / difficulty;
    
    hash_int <= target
}