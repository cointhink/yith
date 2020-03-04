use crate::config;
use crate::exchange;
use crate::types;
use crate::eth;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    blockchain: String,
    contract_hash: String,
    r#type: BuySell,
    pair: String,
    quantity: String,
    price: String,
    address: String,
    signature: String,
    timestamp: u128,
    use_native_tokens: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderForm {
    blockchain: String,
    chain_id: i64,
}

pub struct Switcheo {}

impl exchange::Api for Switcheo {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeApi,
        market: &types::Market,
        offer: &types::Offer,
        proxy: &str,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        println!(
            "Switcheo build {:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );

        let side = match askbid {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        };

        let mut market_pair = make_market_pair(market.swapped, &market.base, &market.quote);

        let now_millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
        let mut sheet = OrderSheet {
            blockchain: "eth".to_string(),
            contract_hash: exchange.contract_address.as_str().to_string(),
            r#type: side,
            pair: market_pair,
            quantity: format!("{}", offer.base_qty),
            price: format!("{}", offer.quote),
            address: format!("0x{}", eth::privkey_to_addr(privkey)),
            signature: "".to_string(),
            timestamp: now_millis,
            use_native_tokens: false,
        };
        sheet.signature = sign(&sheet).to_string();

        let url = format!("{}/orders", exchange.api_url.as_str());
        println!("switcheo limit order build {}", url);
        println!("{:#?}", sheet);
        let client = reqwest::blocking::Client::new();

        let resp = client.post(url.as_str()).json(&sheet).send()?;
        println!("{:#?} {}", resp.status(), resp.url());
        println!("{}", resp.text()?);
        //if resp.status().is_success() {}

        Ok(exchange::OrderSheet::Switcheo(sheet))
    }

    fn submit(&self, sheet: exchange::OrderSheet) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub fn make_market_pair(swapped: bool, base: &types::Ticker, quote: &types::Ticker) -> String {
    match swapped {
        true => format!("{}_{}", quote.symbol, base.symbol),
        false => format!("{}_{}", base.symbol, quote.symbol),
    }
}

pub fn sign<'a>(sheet: &OrderSheet) -> &'a str {
    "sign"
}
