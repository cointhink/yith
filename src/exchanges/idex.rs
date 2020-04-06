use crate::config;
use crate::exchange;
use crate::types;
use reqwest::header;
use serde::{Deserialize, Serialize};
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

pub struct Idex {
    settings: config::ExchangeSettings,
    api_key: String,
}

impl Idex {
    pub fn new(settings: config::ExchangeSettings, config: &config::Config) -> Idex {
        Idex {
            settings: settings,
            api_key: config.idex_key.clone(),
        }
    }

    pub fn build_http_client(&self) -> reqwest::Result<reqwest::blocking::Client> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "API-Key",
            header::HeaderValue::from_str(&self.api_key).unwrap(), //boom
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
        let client = self.build_http_client();
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
}
