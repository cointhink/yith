use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub redis_url: String,
    pub geth_url: String,
    pub wallet_private_key: String,
}

pub fn read_config(filename: &str) -> Config {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let config: Config = serde_yaml::from_str(&yaml).unwrap();
    config
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeApi {
    pub name: String,
    pub enabled: bool,
    pub protocol: ExchangeProtocol,
    pub build_url: String,
    pub order_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExchangeProtocol {
    #[serde(rename = "0x")]
    ZeroexOpen,
    #[serde(rename = "hydro")]
    Hydro,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeList {
    exchanges: Vec<ExchangeApi>,
}

impl ExchangeList {
    pub fn find_by_name(&self, name: &str) -> Option<&ExchangeApi> {
        for api in &self.exchanges {
            if api.name == name {
                return Some(api);
            }
        }
        None
    }
}

pub fn read_exchanges(filename: &str) -> ExchangeList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let config: ExchangeList = serde_yaml::from_str(&yaml).unwrap();
    config
}
