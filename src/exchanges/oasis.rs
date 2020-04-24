use crate::config;
use crate::eth;
use crate::exchange;
use crate::exchanges;
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
    pub fn get(&self, base: &str, quote: &str) -> &PairDetail {
        self.pairs
            .values()
            .find(|pd| pd.base == base && pd.quote == quote)
            .unwrap()
    }
}

pub struct Oasis {
    infura_id: String,
    client: reqwest::blocking::Client,
    pairs: PairList,
    abi: Vec<AbiCall>,
    tokens: exchanges::idex::TokenList, // borrow from Idex
}

impl Oasis {
    pub fn new(settings: config::ExchangeSettings, api_key: &str) -> Oasis {
        let client = Oasis::build_http_client().unwrap();
        let pairs = read_pairs("notes/oasis-pairs.json");
        let abi = read_abi("notes/oasis-abi.json");
        let tokens = exchanges::idex::TokenList::read_tokens("notes/idex-tokens.json");
        Oasis {
            infura_id: api_key.to_string(),
            client: client,
            pairs: pairs,
            abi: abi,
            tokens: tokens,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AbiCall {
    r#type: String,
    name: Option<String>,
}

pub fn read_abi(filename: &str) -> Vec<AbiCall> {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    serde_yaml::from_str::<Vec<AbiCall>>(&yaml).unwrap()
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
        let pub_addr = format!("0x{}", eth::privkey_to_addr(privkey));
        let pair = self.pairs.get(&market.base.symbol, &market.quote.symbol);
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
        let base_token = &self.tokens.get(&pair.base).address;
        let quote_token = &self.tokens.get(&pair.quote).address;
        let order_sheet = match askbid {
            types::AskBid::Ask => OrderSheet {
                address: pub_addr,
                token_buy: base_token.to_string(),
                amount_buy: qty_str,
                token_sell: quote_token.to_string(),
                amount_sell: cost_str,
            },
            types::AskBid::Bid => OrderSheet {
                address: pub_addr,
                token_buy: quote_token.to_string(),
                amount_buy: cost_str,
                token_sell: base_token.to_string(),
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
            tx.insert("from".to_string(), sheet.address.clone());
            let contract_addr = exchange.contract_address.clone();
            tx.insert("to".to_string(), contract_addr);
            let data = eth_data(&sheet);
            tx.insert("data".to_string(), data.to_string());
            //tx.insert("value".to_string(), format!("0x{:x}", 10));
            let params = (tx, Some("latest".to_string()));
            let url = format!("{}/{}", exchange.api_url.as_str(), self.infura_id);
            let resp = geth::rpc(&url, "eth_call", geth::ParamTypes::Infura(params)).unwrap();
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

pub fn eth_data(sheet: &OrderSheet) -> String {
    let mut call = Vec::<u8>::new();
    let func = &eth::hash_msg(&"getMinSell(address)".to_string().as_bytes().to_vec())[0..4];
    call.append(&mut func.to_vec());
    let mut p1 = hex::decode(eth::encode_addr2(&sheet.token_buy)).unwrap();
    call.append(&mut p1);
    format!("0x{}", hex::encode(call))
}
