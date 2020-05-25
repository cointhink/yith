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
