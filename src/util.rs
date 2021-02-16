use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct Measurement {
    pub timestamp: u64,
    pub deg_c: f32,
}

impl PartialEq for Measurement {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && self.deg_c == other.deg_c
    }
}

impl Eq for Measurement {}

impl PartialOrd for Measurement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl Ord for Measurement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
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
