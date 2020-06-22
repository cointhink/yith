use crate::{http_error, http_info};
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

pub const COIN_GECKO_API: &'static str = "https://api.coingecko.com/api/v3";

#[derive(Debug, Serialize, Deserialize)]
pub struct Coin {
    id: String,
    symbol: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceResponse {
    #[serde(flatten)]
    prices: HashMap<String, HashMap<String, f64>>,
}

pub struct CoinGecko {
    coins: Vec<Coin>,
    client: reqwest::blocking::Client,
}

impl CoinGecko {
    pub fn new() -> CoinGecko {
        let client = build_http_client();
        let coins_json = coins_cache(&client);
        let coins = serde_json::from_str::<Vec<Coin>>(&coins_json).unwrap();
        CoinGecko {
            coins: coins,
            client: client,
        }
    }

    pub fn symbol_to_id(&self, symbol: &str) -> &str {
        let winner = self
            .coins
            .iter()
            .find(|c| c.symbol == symbol.to_lowercase());
        match winner {
            Some(coin) => &coin.id,
            None => "none",
        }
    }

    pub fn prices(&self, coin_symbols: Vec<&str>, quote_symbol: &str) -> HashMap<String, f64> {
        let mut coin_mapping: HashMap<&str, &str> = HashMap::new();
        for symbol in coin_symbols {
            coin_mapping.insert(self.symbol_to_id(symbol), symbol);
        }

        let url = format!(
            "{}/simple/price?vs_currencies={}&ids={}",
            COIN_GECKO_API,
            quote_symbol,
            coin_mapping
                .keys()
                .map(|sp| *sp) // such rust
                .collect::<Vec<&str>>()
                .join(",")
        );
        http_info!("{}", url);
        let resp = self.client.get(&url).send().unwrap();
        let price_data = resp.json::<PriceResponse>().unwrap();
        let mut prices: HashMap<String, f64> = HashMap::new();
        for (symbol, quotedata) in price_data.prices {
            prices.insert(
                coin_mapping.get(symbol.as_str()).unwrap().to_string(), //rust wha, &symbol breaks, symbol.as_str() works
                *quotedata.get(quote_symbol).unwrap(),
            );
        }
        prices
    }
}

pub fn coins_json(client: &reqwest::blocking::Client) -> String {
    let url = format!("{}/coins/list", COIN_GECKO_API);
    println!("{}", url);
    let json = client.get(&url).send().unwrap().text().unwrap();
    json
}

pub fn coins_cache(client: &reqwest::blocking::Client) -> String {
    let filename = "notes/coingecko-tokens.json";
    let file_ok = fs::read_to_string(filename);
    match file_ok {
        Ok(yaml) => yaml,
        Err(_e) => {
            let json = coins_json(client);
            fs::write(filename, &json).unwrap();
            json
        }
    }
}

pub fn build_http_client() -> reqwest::blocking::Client {
    let headers = header::HeaderMap::new();
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(headers)
        .build()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup() {}
}
