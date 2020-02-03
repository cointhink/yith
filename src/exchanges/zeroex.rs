use crate::types;
use crate::config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    chain_id: u32,
    exchange_address: String,
}

pub fn build(askbid: &types::AskBid, exchange: &config::ExchangeApi, market: &types::Market, offer: &types::Offer) -> Result<(), Box<dyn std::error::Error>> {
  println!("0x build {:#?} {:#?} {}@{}", askbid, market.source.name, offer.base_qty, offer.quote);
  Ok(())
}

pub fn order(os: OrderSheet) {
  println!("0x order! {:#?}", os);
}

