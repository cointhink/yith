use crate::types;
use crate::config;

pub fn order(askbid: &types::AskBid, exchange: &config::ExchangeApi, market: &types::Market, offer: &types::Offer) {
  println!("0x order! {:#?} {:#?} {}@{}", askbid, market.source.name, offer.base_qty, offer.quote);
}

