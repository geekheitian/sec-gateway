use chrono::{DateTime, Duration, Utc};
use rand::{rngs::OsRng, RngCore};
use std::sync::{Arc, RwLock};

pub struct KeyRotation {
    current_key: Arc<RwLock<[u8; 32]>>,
    last_rotation: Arc<RwLock<DateTime<Utc>>>,
    rotation_interval_days: u64,
}

impl KeyRotation {
    pub fn new(initial_key: [u8; 32], rotation_interval_days: u64) -> Self {
        Self {
            current_key: Arc::new(RwLock::new(initial_key)),
            last_rotation: Arc::new(RwLock::new(Utc::now())),
            rotation_interval_days,
        }
    }

    pub fn should_rotate(&self) -> bool {
        let last_rotation = self.last_rotation.read().unwrap();
        let elapsed = Utc::now() - *last_rotation;
        elapsed > Duration::days(self.rotation_interval_days as i64)
    }

    pub fn rotate_key(&self) -> Result<[u8; 32], String> {
        let mut new_key = [0u8; 32];
        OsRng.fill_bytes(&mut new_key);

        let mut current = self
            .current_key
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        *current = new_key;

        let mut last = self
            .last_rotation
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        *last = Utc::now();

        Ok(new_key)
    }

    pub fn current_key(&self) -> Result<[u8; 32], String> {
        self.current_key
            .read()
            .map(|k| *k)
            .map_err(|e| format!("Failed to acquire read lock: {}", e))
    }

    pub fn last_rotation_time(&self) -> Result<DateTime<Utc>, String> {
        self.last_rotation
            .read()
            .map(|t| *t)
            .map_err(|e| format!("Failed to acquire read lock: {}", e))
    }
}
