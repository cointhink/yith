use crate::config;
use crate::types;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    r#type: String,
    quantity: f64,
    price: f64,
    expiration: f64,
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
    }
    let sheet = OrderSheet {
        r#type: "UserOrderType".to_string(),
        quantity: qty,
        price: price,
        expiration: 500.0,
    };
    let mut url = exchange.build_url.clone();
    url.push_str(format!("/markets/{}/order/limit", market_id).as_str());
    println!("0x order {}", url);
    println!("{:#?}", sheet);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(exchange.build_url.as_str())
        .json(&sheet);
        //.send()?;
    //let body = resp.json::<HashMap<String, String>>()?;
    Ok(())
}

pub fn order(os: OrderSheet) {
    println!("0x order! {:#?}", os);
}

pub fn make_market_id(swapped: bool, base: &types::Ticker, quote: &types::Ticker) -> String {
    match swapped {
        true => format!("{}-{}", quote.symbol, base.symbol),
        false => format!("{}-{}", base.symbol, quote.symbol),
    }
}
