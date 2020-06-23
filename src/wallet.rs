use crate::price;
use crate::time;
use crate::types;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet {
    pub coins: Vec<WalletCoin>,
}

impl Wallet {
    pub fn reset(&mut self) {
        self.coins.retain(|c| c.source == "limit");
    }

    pub fn coin_limit(&self, name: &str) -> f64 {
        match self.find_coin_by_symbol(name) {
            Ok(coin) => coin.amounts[0].base_qty,
            Err(_msg) => 0.0,
        }
    }

    pub fn find_coin_by_symbol(&self, symbol: &str) -> Result<&WalletCoin, WalletError> {
        self.find_coin_by_source_symbol("limit", symbol)
    }

    pub fn find_coin_by_source_symbol(
        &self,
        source: &str,
        symbol: &str,
    ) -> Result<&WalletCoin, WalletError> {
        for coin in &self.coins {
            if coin.ticker_symbol == symbol && coin.source == source {
                return Ok(&coin);
            }
        }
        Err(WalletError {})
    }

    pub fn print_with_price(&self) {
        let coin_gecko = price::CoinGecko::new();
        let coin_ids = self
            .coins
            .iter()
            .map(|c| c.ticker_symbol.as_ref())
            .collect::<Vec<&str>>();
        let quote_symbol = "usd";
        let prices = coin_gecko.prices(coin_ids, quote_symbol);
        println!("[wallet {}]", time::now_string());
        let mut subtotals: HashMap<&str, f64> = HashMap::new();
        for coin in &self.coins {
            if coin.source != "limit" {
                let percoin = prices.get(&coin.ticker_symbol).unwrap();
                let quote_total = coin.base_total() * percoin;
                let source = coin.source.as_ref(); // rust hashmap ptr wha
                if !subtotals.contains_key(source) {
                    subtotals.insert(source, 0.0);
                }
                subtotals.insert(source, subtotals.get(source).unwrap() + quote_total);
                println!("{} {:5.2}{}", coin, quote_total, quote_symbol);
            }
        }
        let mut total = 0.0;
        for (source, subtotal) in subtotals {
            println!("{:8.8} = {:0.5}{}", source, subtotal, quote_symbol);
            total = total + subtotal;
        }
        println!("*Total   = {:0.5}{}", total, quote_symbol);
    }
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
            "{:8.8}:{:8.5}:{:4}({:>8.8})",
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

#[derive(Debug)]
pub struct WalletError {}
impl std::error::Error for WalletError {}
impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WalletError is here!")
    }
}
