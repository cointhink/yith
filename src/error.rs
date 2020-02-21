#[derive(Debug)]
pub struct OrderError {
    details: String,
}

impl OrderError {
    pub fn new(msg: &str) -> OrderError {
        OrderError {
            details: msg.to_string(),
        }
    }
}

impl std::error::Error for OrderError {}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.details)
    }
}
