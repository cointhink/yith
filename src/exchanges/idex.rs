use crate::config;
use crate::exchange;
use crate::types;
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    blockchain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    #[serde(flatten)]
    balances: HashMap<String, String>,
}

pub struct Idex {
    settings: config::ExchangeSettings,
    client: reqwest::blocking::Client,
}

impl Idex {
    pub fn new(settings: config::ExchangeSettings, api_key: &str) -> Idex {
        let client = Idex::build_http_client(api_key).unwrap();
        Idex {
            settings: settings,
            client: client,
        }
    }

    pub fn build_http_client(api_key: &str) -> reqwest::Result<reqwest::blocking::Client> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "API-Key",
            header::HeaderValue::from_str(api_key).unwrap(), //boom
        );
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()
    }
}

impl exchange::Api for Idex {
    fn setup(&mut self) {}

    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &exchange::Market,
        offer: &types::Offer,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        Ok(exchange::OrderSheet::Idex(OrderSheet {
            blockchain: "eth".to_string(),
        }))
    }

    fn submit(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn balances<'a>(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> HashMap<String, f64> {
        let url = format!(
            "{}/returnBalances?address=0x{}",
            exchange.api_url.as_str(),
            public_addr
        );
        let resp = self.client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        let response = resp.json::<BalanceResponse>().unwrap();
        response
            .balances
            .iter()
            .map(|(symbol, strval)| {
                let f64 = strval.parse::<f64>().unwrap();
                (symbol.clone(), f64)
            })
            .collect()
    }
}
