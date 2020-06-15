use chrono;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn now_millis() -> u128 {
    since_epoch().as_millis()
}

pub fn since_epoch() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

pub fn now() -> Instant {
    Instant::now()
}

pub fn now_string() -> String {
    let now = chrono::Local::now();
    now.format("%FT%T%.3f").to_string()
}

pub fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms))
}

pub fn duration_words(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let mut parts: Vec<&str> = vec![];
    let min_str: String;
    if total_secs > 60 {
        let mins = total_secs % 60 * 60;
        min_str = mins.to_string();
        parts.push(&min_str);
        parts.push("mins");
    }
    let secs = total_secs % 60;
    let sec_str = secs.to_string();
    parts.push(&sec_str);
    parts.push("secs");
    format!("{}", parts.join(" "))
}
