use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms))
}
