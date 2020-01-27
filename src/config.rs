use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub redis: String,
}

pub fn load() -> Config {
    let filename = "config.yaml";
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let config: Config = serde_yaml::from_str(&yaml).unwrap();
    println!("{:#?}", config);
    config
}

