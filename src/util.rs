use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct Measurement {
    pub deg_c: f32,
    pub timestamp: u64
}

impl Measurement {
    pub fn new(deg_c: u16) -> Self {
        Self {
            deg_c: deg_c as f32 / 10.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }
    }
}
