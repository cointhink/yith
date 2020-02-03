use crate::types;
use crate::config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    Buy, Sell
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LimitMarket {
    Limit, Market
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    market_id: String,
    side: BuySell,
    order_type: LimitMarket,
}

pub fn build(askbid: &types::AskBid, exchange: &config::ExchangeApi, market: &types::Market, offer: &types::Offer) -> Result<(), Box<dyn std::error::Error>> {
  println!("HYDRO build {:#?} {:#?} {}@{}", askbid, market.source.name, offer.base_qty, offer.quote);
  let sheet = OrderSheet{market_id: "A".to_string(), side: BuySell::Buy, order_type: LimitMarket::Limit};
  let url = "";
  let client = reqwest::blocking::Client::new();
  let resp = client.post(url).json(&sheet).send()?;
  let body = resp.json::<HashMap<String, String>>()?;
  Ok(())
}

pub fn order(os: OrderSheet) {
  println!("HYDRO order! {:#?}", os);
}

