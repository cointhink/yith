use crate::types;
use crate::config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    market_id: String,
    side: String,
}

pub fn build(askbid: &types::AskBid, exchange: &config::ExchangeApi, market: &types::Market, offer: &types::Offer) {
  println!("HYDRO build {:#?} {:#?} {}@{}", askbid, market.source.name, offer.base_qty, offer.quote);
}

pub fn order(os: OrderSheet) {
  println!("HYDRO order! {:#?}", os);
}

