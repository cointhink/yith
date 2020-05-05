use chrono;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now_millis() -> u128 {
    now().as_millis()
}

pub fn now() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

pub fn now_string() -> String {
    let now = chrono::Local::now();
    now.format("%FT%T%.3f").to_string()
}

pub fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms))
}
