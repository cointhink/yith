use crate::exchange;
use crate::types;
use crate::config;
use serde::{Deserialize, Serialize};

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
    quantity: String,
    price: String,
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Switcheo build {:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );

        let side = match askbid {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        };

        let sheet = OrderSheet {
        	blockchain: "eth".to_string(),
        	contract_hash: exchange.contract_address.as_str().to_string(),
            r#type: side,
            quantity: format!("{}", offer.base_qty),
            price: format!("{}", offer.quote),
        };

        let url = format!(
            "{}/orders",
            exchange.api_url.as_str());
        println!("switcheo limit order build {}", url);
        println!("{:#?}", sheet);
        let client = reqwest::blocking::Client::new();

        let resp = client.post(url.as_str()).json(&sheet).send()?;
        println!("{:#?} {}", resp.status(), resp.url());
        if resp.status().is_success() {
        }

    	Ok(())
    }
}