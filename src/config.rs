use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub redis_url: String,
}

pub fn read_config(filename: &str) -> Config {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let config: Config = serde_yaml::from_str(&yaml).unwrap();
    config
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeApi {
    name: String,
    enabled: bool,
    protocol: ExchangeProtocol,
    build_url: String,
    order_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExchangeProtocol { zeroex, hydro, ddex3 }

pub fn read_exchanges(filename: &str) -> Vec<ExchangeApi> {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let config: Vec<ExchangeApi> = serde_yaml::from_str(&yaml).unwrap();
    config
}
