use crate::config;
use crate::eth;
use crate::exchange;
use crate::types;
use serde::{Deserialize, Serialize};
use secp256k1::{SecretKey};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
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
    side: BuySell,
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
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
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
            contract_hash: exchange.contract_address[2..].to_string(),
            side: side,
            pair: market_pair,
            quantity: format!("{}", offer.base_qty),
            price: format!("{}", offer.quote),
            address: format!("{}", eth::privkey_to_addr(privkey)),
            timestamp: now_millis,
            use_native_tokens: false,
        };
        let json = serde_json::to_string(&sheet).unwrap();
        let signature = sign(&json, &secret_key).to_string();
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

pub fn sign<'a>(json: &String, secret_key: &SecretKey) -> String {
    println!("json {}", json);
    let msg_hash = eth::hash_msg(&json.as_bytes().to_vec());
    println!("hash {}", hex::encode(msg_hash));
    let sig_bytes = eth::sign_bytes(&msg_hash, &secret_key);
    format!("0x{}", hex::encode(sig_bytes.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    static privkey: &str = "98c193239bff9eb53a83e708b63b9c08d6e47900b775402aca2acc3daad06f24";
                           
    #[test]
    fn test_order_sign() {
        let json = "{\"apple\":\"Z\",\"blockchain\":\"eth\",\"timestamp\":1529380859}";
        println!("privkey {} {}", &privkey, &json);
        let privkey_bytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privkey_bytes).unwrap();
        let signature = sign(&json.to_string(), &secret_key);
        println!("json sign signature {}", signature);
        let good_sig = "0xbcff177dba964027085b5653a5732a68677a66c581f9c85a18e1dc23892c72d86c0b65336e8a17637fd1fe1def7fa8cbac43bf9a8b98ad9c1e21d00e304e32911c";                        
        assert_eq!(signature, good_sig)
    }
}

/*
>  web3.eth.accounts.sign('{"apple":"Z","blockchain":"eth","timestamp":1529380859}', 
                  '0x98c193239bff9eb53a83e708b63b9c08d6e47900b775402aca2acc3daad06f24')
{ message: '{"apple":"Z","blockchain":"eth","timestamp":1529380859}',
  messageHash: '0xd912c2d8ddef5f07bfa807be8ddb4d579ab978f52ab1176deea8b260f146ea21',
  v: '0x1c',
  r: '0xbcff177dba964027085b5653a5732a68677a66c581f9c85a18e1dc23892c72d8',
  s: '0x6c0b65336e8a17637fd1fe1def7fa8cbac43bf9a8b98ad9c1e21d00e304e3291',
  signature: '0xbcff177dba964027085b5653a5732a68677a66c581f9c85a18e1dc23892c72d86c0b65336e8a17637fd1fe1def7fa8cbac43bf9a8b98ad9c1e21d00e304e32911c' }
*/
