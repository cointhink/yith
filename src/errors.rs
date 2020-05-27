use std::error;
use std::fmt;

#[derive(Debug)]
pub struct MainError {
    pub msg: String,
}

impl MainError {
    pub fn build_box(msg: String) -> Box<dyn error::Error> {
        println!("{}", msg);
        Box::new(MainError { msg: msg })
    }
}

impl error::Error for MainError {
    fn description(&self) -> &str {
        &self.msg
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
