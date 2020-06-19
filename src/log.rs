pub use log::{debug, error, info, trace, warn};
use log4rs;

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! http_error {
        ($($arg:tt)*) => (
            log::error!(target: "http", $($arg)*);
        )
    }
    #[macro_export]
    macro_rules! http_info {
        ($($arg:tt)*) => (
            log::info!(target: "http", $($arg)*);
        )
    }
}

pub fn init() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
}

pub struct RunLog {
    lines: Vec<String>,
}

impl RunLog {
    pub fn new() -> RunLog {
        RunLog { lines: Vec::new() }
    }

    pub fn add(&mut self, line: String) {
        println!("{}", line);
        self.lines.push(line);
    }
}

impl std::fmt::Display for RunLog {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.lines.join("\n"))
    }
}
