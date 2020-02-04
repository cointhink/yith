use crate::config;
use crate::types;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    chain_id: u32,
    exchange_address: String,
}

pub fn build(
    askbid: &types::AskBid,
    exchange: &config::ExchangeApi,
    market: &types::Market,
    offer: &types::Offer,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "HYDRO build {:#?} {} {}@{}",
        askbid, market, offer.base_qty, offer.quote
    );
    let mut qty = offer.base_qty;
    let mut price = offer.quote;
    if market.swapped {
        let ab = askbid.swap();
        let (s_qty, s_price) = offer.swap();
        println!(
            "unswapped {:#?} {} {}-{} {}@{}",
            ab, market.source.name, market.quote, market.base, s_qty, s_price
        );
    }
    Ok(())
}

pub fn order(os: OrderSheet) {
    println!("0x order! {:#?}", os);
}
