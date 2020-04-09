use crate::exchange;
use crate::exchanges;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub redis_url: String,
    pub geth_url: String,
    pub wallet_private_key: String,
    pub proxy: Option<String>,
    pub etherscan_key: String,
    pub idex_key: String,
    pub email: Option<String>,
}

pub fn read_config(filename: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(filename)?;
    let config: Config = serde_yaml::from_str(&yaml)?;
    Ok(config)
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

pub fn read_exchanges(
    filename: &str,
    config: &Config,
) -> Result<ExchangeList, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(filename)?;
    let exchange_settings: Vec<ExchangeSettings> = serde_yaml::from_str(&yaml)?;
    let elist: Vec<Exchange> = vec![];
    let mut list = ExchangeList { exchanges: elist };
    for settings in exchange_settings.into_iter() {
        let api: Box<dyn exchange::Api> = match settings.protocol {
            ExchangeProtocol::ZeroexOpen => Box::new(exchanges::zeroex::Zeroex {}),
            ExchangeProtocol::Ddex3 => Box::new(exchanges::ddex3::Ddex3::new(settings.clone())),
            ExchangeProtocol::Ddex4 => Box::new(exchanges::ddex4::Ddex4 {}),
            ExchangeProtocol::Switcheo => {
                Box::new(exchanges::switcheo::Switcheo::new(settings.clone()))
            }
            ExchangeProtocol::Idex => Box::new(exchanges::idex::Idex::new(
                settings.clone(),
                &config.idex_key,
            )),
        };
        list.exchanges.push(Exchange {
            api: api,
            settings: settings,
        });
    }
    Ok(list)
}

type TokenList = exchanges::switcheo::TokenList;
type Token = exchanges::switcheo::TokenDetail;

pub fn read_tokens( filename: &str ) -> TokenList {
    let yaml = fs::read_to_string(filename).unwrap();
    let tokens = serde_yaml::from_str(&yaml).unwrap();
    TokenList { tokens: tokens }
}
