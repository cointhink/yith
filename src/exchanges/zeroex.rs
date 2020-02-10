use crate::config;
use crate::types;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    r#type: BuySell,
    quantity: String,
    price: String,
    expiration: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderForm {
  chain_id: i64,
  maker_address: String,
  signature: String,
  sender_address: String,
  exchange_address: String,
  taker_address: String,
  maker_fee: String,
  taker_fee: String,
  maker_fee_asset_data: String,
  taker_fee_asset_data: String,
  maker_asset_amount: String,
  taker_asset_amount: String,
  maker_asset_data: String,
  taker_asset_data: String,
  salt: String,
  fee_recipient_address: String,
  expiration_time_seconds: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    status: i64,
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

pub fn build(
    privkey: &str,
    askbid: &types::AskBid,
    exchange: &config::ExchangeApi,
    market: &types::Market,
    offer: &types::Offer,
    proxy: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "HYDRO build {:#?} {} {}@{}",
        askbid, market, offer.base_qty, offer.quote
    );
    let mut market_id = make_market_id(market.swapped, &market.base, &market.quote);
    let mut qty = offer.base_qty;
    let mut price = offer.quote;
    let mut ab = askbid;
    let askbid_other = askbid.otherside();
    if market.swapped {
        ab = &askbid_other;
        let (s_qty, s_price) = offer.swap();
        println!(
            "unswapped {:#?} {} {}-{} {}@{}",
            ab, market.source.name, market.quote, market.base, s_qty, s_price
        );
    }
    let side = match ab {
        types::AskBid::Ask => BuySell::Buy,
        types::AskBid::Bid => BuySell::Sell,
    };
    let expire_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let sheet = OrderSheet {
        r#type: side,
        quantity: format!("{}", qty),
        price: format!("{}", price),
        expiration: format!("{}", expire_time),
    };
    let mut url = exchange.build_url.clone();
    url.push_str(format!("/{}/order/limit", market_id).as_str());
    println!("0x order {}", url);
    println!("{:#?}", sheet);
    let client = reqwest::blocking::Client::new();
    let resp = client.post(url.as_str()).json(&sheet).send()?;
    println!("{:#?} {}", resp.status(), resp.url());
    if resp.status().is_success() {
        let form = resp.json::<OrderForm>().unwrap();
        println!("{:#?}", form); } else {
        let body = resp.json::<BuildResponse>().unwrap();
        println!("{:#?}", body); 
    }
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
