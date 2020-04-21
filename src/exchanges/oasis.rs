use crate::config;
use crate::exchange;
use crate::geth;
use crate::types;
use serde::{Deserialize, Serialize};
use std::collections;
use std::error;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    pub address: String,
    pub token_buy: String,
    pub amount_buy: String,
    pub token_sell: String,
    pub amount_sell: String,
}

pub struct Oasis {
    infura_id: String,
    client: reqwest::blocking::Client,
}

impl Oasis {
    pub fn new(settings: config::ExchangeSettings, api_key: &str) -> Oasis {
        let client = Oasis::build_http_client().unwrap();
        Oasis {
            infura_id: api_key.to_string(),
            client: client,
        }
    }

    pub fn build_http_client() -> reqwest::Result<reqwest::blocking::Client> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
    }
}

impl exchange::Api for Oasis {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &exchange::Market,
        offer: &types::Offer,
    ) -> Result<exchange::OrderSheet, Box<dyn error::Error>> {
        let order_sheet = OrderSheet {
            address: "".to_string(),
            token_buy: "".to_string(),
            amount_buy: "".to_string(),
            token_sell: "".to_string(),
            amount_sell: "".to_string(),
        };
        Ok(exchange::OrderSheet::Oasis(order_sheet))
    }

    fn submit(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<(), Box<dyn error::Error>> {
        let mut tx = geth::JsonRpcParam::new();
        tx.insert("from".to_string(), "0x12".to_string());
        let params = vec![tx];
        let rpc = geth::JsonRpc {
            jsonrpc: "2.0".to_string(),
            id: "12".to_string(),
            method: "eth_call".to_string(),
            params: params,
        };
        let url = format!("{}/{}", exchange.api_url.as_str(), self.infura_id);
        println!("{:?}", url);
        let resp = self.client.post(url.as_str()).json(&rpc).send().unwrap();
        println!("{} {}", resp.status(), resp.text().unwrap());
        Ok(())
    }

    fn balances<'a>(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceList {
        collections::HashMap::new()
    }
}
