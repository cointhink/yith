use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now_millis() -> u128 {
    now().as_millis()
}

pub fn now_secs() -> u64 {
    now().as_secs()
}

pub fn now() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

pub fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms))
}
