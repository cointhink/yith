#[derive(Debug)]
pub struct MainError {
    pub msg: String,
}

impl MainError {
    pub fn build_box(msg: String) -> Box<dyn std::error::Error> {
        println!("{}", msg);
        Box::new(MainError { msg: msg })
    }
}

impl std::error::Error for MainError {
    fn description(&self) -> &str {
        &self.msg
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl std::fmt::Display for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
