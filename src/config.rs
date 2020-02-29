use crate::types;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub redis_url: String,
    pub geth_url: String,
    pub wallet_private_key: String,
    pub proxy: String,
    pub etherscan_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet {
    pub coins: Vec<WalletCoin>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCoin {
    pub ticker_symbol: String,
    pub amounts: Vec<types::Offer>,
}

pub fn read_config(filename: &str) -> Config {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let config: Config = serde_yaml::from_str(&yaml).unwrap();
    config
}

pub fn read_wallet(filename: &str) -> Wallet {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let wallet: Wallet = serde_yaml::from_str(&yaml).unwrap();
    wallet
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeApi {
    pub name: String,
    pub enabled: bool,
    pub protocol: ExchangeProtocol,
    pub contract_address: String,
    pub api_url: String,
    pub maker_fee: f64,
    pub taker_fee: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExchangeProtocol {
    #[serde(rename = "0x")]
    ZeroexOpen,
    #[serde(rename = "ddex3")]
    Ddex3,
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

impl Wallet {
    pub fn coin_amount(&self, name: &str) -> f64 {
        match self.find_coin(name) {
            Ok(coin) => coin.amounts[0].base_qty,
            Err(_msg) => 0.0,
        }
    }

    pub fn find_coin(&self, name: &str) -> Result<&WalletCoin, &'static str> {
        for coin in &self.coins {
            if coin.ticker_symbol == name {
                return Ok(&coin);
            }
        }
        Err("not found")
    }
}

impl WalletCoin {
    pub fn base_total(&self) -> f64 {
        self.amounts
            .iter()
            .fold(0.0, |acc, coin| acc + coin.base_qty)
    }
}

impl fmt::Display for WalletCoin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:0.5} {}.", self.base_total(), self.ticker_symbol)
    }
}

impl fmt::Display for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wallet: {} coins. ", self.coins.len())?;
        self.coins.iter().try_for_each(|c| {
            write!(f, "{}", c)
        });
        write!(f, "")
    }
}
