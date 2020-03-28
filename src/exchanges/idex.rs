use crate::config;
use crate::eth;
use crate::exchange;
use crate::types;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::time;

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

pub struct Idex {}

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
}
