use crate::exchange;
use crate::exchanges;
use crate::geth;
use once_cell::sync::OnceCell;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;

pub static FILENAME: &'static str = "config.yaml";
pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub redis_url: String,
    pub geth_url: String,
    pub wallet_private_key: String,
    pub proxy: Option<String>,
    pub etherscan_key: String,
    pub idex_key: String,
    pub infura_project_id: String,
    pub email: Option<String>,
    pub spread_premium: Option<f64>,
}

pub fn read_type<T>(filename: &str) -> T
where
    T: DeserializeOwned,
{
    let yaml = fs::read_to_string(filename).unwrap_or_else(|err| panic!("{} {}", filename, err));
    let obj: T = serde_yaml::from_str(&yaml).unwrap_or_else(|err| panic!("{} {}", filename, err));
    obj
}

pub struct Exchange {
    pub settings: ExchangeSettings,
    pub api: Box<dyn exchange::Api>,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.settings.name)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExchangeSettings {
    pub name: String,
    pub enabled: bool,
    pub has_balances: bool,
    pub protocol: ExchangeProtocol,
    pub contract_address: String,
    pub api_url: String,
    pub maker_fee: f64,
    pub taker_fee: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExchangeProtocol {
    #[serde(rename = "0x")]
    ZeroexOpen,
    #[serde(rename = "ddex3")]
    Ddex3,
    #[serde(rename = "ddex4")]
    Ddex4,
    #[serde(rename = "switcheo")]
    Switcheo,
    #[serde(rename = "idex")]
    Idex,
    #[serde(rename = "oasis")]
    Oasis,
}

pub struct ExchangeList {
    pub exchanges: Vec<Exchange>,
}

impl ExchangeList {
    pub fn find_by_name(&self, name: &str) -> Option<&Exchange> {
        for exg in &self.exchanges {
            if exg.settings.name == name {
                return Some(exg);
            }
        }
        None
    }

    pub fn enabled(&self) -> Vec<&Exchange> {
        self.exchanges
            .iter()
            .filter(|e| e.settings.enabled)
            .collect()
    }
}

pub fn hydrate_exchanges(
    filename: &str,
    config: &Config,
) -> Result<ExchangeList, Box<dyn std::error::Error>> {
    let exchange_settings: Vec<ExchangeSettings> = read_type(filename);
    let exchanges = exchange_settings
        .into_iter()
        .map(|settings| {
            let api: Box<dyn exchange::Api> = match settings.protocol {
                ExchangeProtocol::ZeroexOpen => Box::new(exchanges::zeroex::Zeroex {}),
                ExchangeProtocol::Ddex3 => Box::new(exchanges::ddex3::Ddex3::new(settings.clone())),
                ExchangeProtocol::Ddex4 => Box::new(exchanges::ddex4::Ddex4 {}),
                ExchangeProtocol::Switcheo => Box::new(exchanges::switcheo::Switcheo::new(
                    settings.clone(),
                    geth::Client::build_infura(&config.infura_project_id),
                )),
                ExchangeProtocol::Idex => Box::new(exchanges::idex::Idex::new(
                    settings.clone(),
                    &config.idex_key,
                    geth::Client::build_infura(&config.infura_project_id),
                )),
                ExchangeProtocol::Oasis => Box::new(exchanges::oasis::Oasis::new(
                    geth::Client::build_infura(&config.infura_project_id),
                )),
            };
            Exchange {
                api: api,
                settings: settings,
            }
        })
        .collect();
    Ok(ExchangeList {
        exchanges: exchanges,
    })
}
