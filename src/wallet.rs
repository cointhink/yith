use crate::types;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet {
    pub coins: Vec<WalletCoin>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCoin {
    pub ticker_symbol: String,
    pub contract: String,
    pub source: String,
    pub amounts: Vec<types::Offer>,
}

impl WalletCoin {
    pub fn build(ticker: &str, contract: &str, name: &str, balance: f64) -> WalletCoin {
        WalletCoin {
            ticker_symbol: ticker.to_string(),
            contract: contract.to_string(),
            source: name.to_string(),
            amounts: vec![types::Offer {
                base_qty: balance,
                quote: 1.0,
            }],
        }
    }
}

impl Wallet {
    pub fn load_file(filename: &str) -> Wallet {
        let file_ok = fs::read_to_string(filename);
        let yaml = file_ok.unwrap();
        let wallet: Wallet = serde_yaml::from_str(&yaml).unwrap();
        wallet
    }

    pub fn coin_amount(&self, name: &str) -> f64 {
        match self.find_coin_by_symbol(name) {
            Ok(coin) => coin.amounts[0].base_qty,
            Err(_msg) => 0.0,
        }
    }

    pub fn find_coin_by_symbol(&self, symbol: &str) -> Result<&WalletCoin, &'static str> {
        self.find_coin_by_source_symbol("limit", symbol)
    }

    pub fn find_coin_by_source_symbol(
        &self,
        source: &str,
        symbol: &str,
    ) -> Result<&WalletCoin, &'static str> {
        for coin in &self.coins {
            if coin.ticker_symbol == symbol && coin.source == source {
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
        write!(
            f,
            "{}:{:0.5}:{}({})",
            self.source,
            self.base_total(),
            self.ticker_symbol,
            self.contract
        )
    }
}

impl fmt::Display for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[wallet]\n")?;
        self.coins.iter().try_for_each(|c| write!(f, "{}\n", c))?;
        write!(f, "")
    }
}
