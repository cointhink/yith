use crate::config;
use crate::eth;
use crate::exchange;
use crate::types;
use serde::{Deserialize, Serialize};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
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
pub struct OrderSheetSign {
    #[serde(flatten)]
    sheet: OrderSheet,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    address: String,
    blockchain: String,
    contract_hash: String,
    pair: String,
    price: String,
    quantity: String,
    r#type: BuySell,
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

        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).expect("32 bytes, within curve order");
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
            timestamp: now_millis,
            use_native_tokens: false,
        };
        let signature = sign(&sheet, &secret_key).to_string();
        let sheet_sign = OrderSheetSign { sheet: sheet, signature: signature };

        let url = format!("{}/orders", exchange.api_url.as_str());
        println!("switcheo limit order build {}", url);
        println!("{:#?}", sheet_sign);
        let client = reqwest::blocking::Client::new();
        let resp = client.post(url.as_str()).json(&sheet_sign).send().unwrap();
        println!("switcheo result {:#?} {}", resp.status(), resp.url());
        println!("{}", resp.text()?);
        //if resp.status().is_success() {}

        Ok(exchange::OrderSheet::Switcheo(sheet_sign))
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

pub fn sign<'a>(sheet: &OrderSheet, secret_key: &SecretKey) -> String {
    let json = serde_json::to_string(sheet).unwrap();
    let msg_hash = eth::ethsign_hash_msg(&json.as_bytes().to_vec());
    let sig_bytes = eth::sign_bytes(&msg_hash, &secret_key);
    format!("{}", hex::encode(sig_bytes.to_vec()))
}
