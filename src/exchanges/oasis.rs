use crate::config;
use crate::eth;
use crate::exchange;
use crate::geth;
use crate::types;
use serde::{Deserialize, Serialize};
use std::collections;
use std::collections::HashMap;
use std::error;
use std::fs;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    pub address: String,
    pub token_buy: String,
    pub amount_buy: String,
    pub token_sell: String,
    pub amount_sell: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairDetail {
    pub base: String,
    pub quote: String,
    pub base_precision: i32,
    pub quote_precision: i32,
    pub active: bool,
}

pub struct PairList {
    pub pairs: HashMap<String, PairDetail>,
}

impl PairList {
    pub fn get(&self, symbol2: &str) -> &PairDetail {
        self.pairs
            .iter()
            .find(|(symbol, detail)| *symbol == symbol2)
            .unwrap()
            .1
    }
}

pub struct Oasis {
    infura_id: String,
    client: reqwest::blocking::Client,
    pairs: PairList,
}

impl Oasis {
    pub fn new(settings: config::ExchangeSettings, api_key: &str) -> Oasis {
        let client = Oasis::build_http_client().unwrap();
        let pairs = read_pairs("notes/oasis-pairs.json");
        Oasis {
            infura_id: api_key.to_string(),
            client: client,
            pairs: pairs,
        }
    }

    pub fn build_http_client() -> reqwest::Result<reqwest::blocking::Client> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
    }
}

pub fn read_pairs(filename: &str) -> PairList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let pairs = serde_yaml::from_str::<HashMap<String, PairDetail>>(&yaml).unwrap();
    PairList { pairs: pairs }
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
        let pub_addr = format!("0x{}",eth::privkey_to_addr(privkey));
        let pair = self.pairs.get(&make_market_pair(market));
        let decimals = 18;
        let cost_int = exchange::quantity_in_base_units(
            offer.cost(*askbid),
            pair.base_precision,
            pair.base_precision,
        );
        let cost_str = cost_int.to_str_radix(10);
        let qty_int = exchange::quantity_in_base_units(
            offer.base_qty,
            pair.quote_precision,
            pair.quote_precision,
        );
        let qty_str = qty_int.to_str_radix(10);
        let order_sheet = match askbid {
            types::AskBid::Ask => OrderSheet {
                address: pub_addr,
                token_buy: market.base_contract.clone(),
                amount_buy: qty_str,
                token_sell: market.quote_contract.clone(),
                amount_sell: cost_str,
            },
            types::AskBid::Bid => OrderSheet {
                address: pub_addr,
                token_buy: market.quote_contract.clone(),
                amount_buy: cost_str,
                token_sell: market.base_contract.clone(),
                amount_sell: qty_str,
            },
        };
        println!("{:?}", order_sheet);
        Ok(exchange::OrderSheet::Oasis(order_sheet))
    }

    fn submit(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        sheet_opt: exchange::OrderSheet,
    ) -> Result<(), Box<dyn error::Error>> {
        if let exchange::OrderSheet::Oasis(sheet) = sheet_opt {
            let mut tx = geth::JsonRpcParam::new();
            tx.insert("from".to_string(), sheet.address);
            let params = vec![tx];
            let rpc = geth::JsonRpc {
                jsonrpc: "2.0".to_string(),
                id: "12".to_string(),
                method: "eth_call".to_string(),
                params: params,
            };
            let url = format!("{}/{}", exchange.api_url.as_str(), self.infura_id);
            println!("{}", serde_json::to_string(&rpc).unwrap());
            println!("{:?}", url);
            let resp = self.client.post(url.as_str()).json(&rpc).send().unwrap();
            println!("{} {}", resp.status(), resp.text().unwrap());
            Ok(())
        } else {
            let order_error = exchange::OrderError {
                msg: "wrong order type passed to submit".to_string(),
                code: 12 as i32,
            };
            println!("ERR: {}", order_error);
            Err(Box::new(order_error))
        }
    }

    fn balances<'a>(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceList {
        collections::HashMap::new()
    }
}

pub fn make_market_pair(market: &exchange::Market) -> String {
    format!("{}/{}", market.base.symbol, market.quote.symbol)
}
