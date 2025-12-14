use std::sync::atomic::{AtomicU64, Ordering};

pub struct SequenceTracker {
    value: AtomicU64,
}

impl SequenceTracker {
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    pub fn update(&self, seq: u64) {
        self.value.store(seq, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}
