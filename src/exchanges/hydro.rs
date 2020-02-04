use crate::config;
use crate::types;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LimitMarket {
    Limit,
    Market,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    market_id: String,
    side: BuySell,
    order_type: LimitMarket,
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
    let mut market_id = make_market_id(market.swapped, &market.base, &market.quote);
    let mut qty = offer.base_qty;
    let mut price = offer.quote;
    if market.swapped {
        let ab = askbid.otherside();
        let (s_qty, s_price) = offer.swap();
        println!(
            "unswapped {:#?} {} {}-{} {}@{}",
            ab, market.source.name, market.quote, market.base, s_qty, s_price
        );
        qty = s_qty;
        price = s_price;
    }
    let sheet = OrderSheet {
        market_id: market_id,
        side: BuySell::Buy,
        order_type: LimitMarket::Limit,
    };
    println!("Hydro order {:#?}", sheet);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(exchange.build_url.as_str())
        .json(&sheet);
        //.send()?;
    //let body = resp.json::<HashMap<String, String>>()?;
    Ok(())
}

pub fn order(os: OrderSheet) {
    println!("HYDRO order! {:#?}", os);
}

pub fn make_market_id(swapped: bool, base: &types::Ticker, quote: &types::Ticker) -> String {
    match swapped {
        true => format!("{}-{}", quote.symbol, base.symbol),
        false => format!("{}-{}", base.symbol, quote.symbol),
    }
}
