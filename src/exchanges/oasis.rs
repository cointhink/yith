use crate::config;
use crate::eth;
use crate::exchange;
use crate::exchanges;
use crate::geth;
use crate::types;
use ethereum_types;
use serde::{Deserialize, Serialize};
use std::collections;
use std::collections::HashMap;
use std::error;
use std::fs;

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
    pairs: PairList,
    contract: Contract,
    tokens: exchanges::idex::TokenList, // borrow from Idex
}

impl Oasis {
    pub fn new(api_key: &str) -> Oasis {
        let pairs = read_pairs("notes/oasis-pairs.json");
        let abi = read_abi("notes/oasis-abi.json");
        let tokens = exchanges::idex::TokenList::read_tokens("notes/oasis-idex-tokens.json");
        Oasis {
            infura_id: api_key.to_string(),
            pairs: pairs,
            contract: abi,
            tokens: tokens,
        }
    }

    fn min_sell(
        &self,
        token: &str,
        exchange: &config::ExchangeSettings,
    ) -> Result<geth::ResultTypes, Box<dyn std::error::Error>> {
        let mut tx = geth::JsonRpcParam::new();
        tx.insert("to".to_string(), exchange.contract_address.clone());
        tx.insert("data".to_string(), get_min_sell_data(token));
        let params = (tx.clone(), Some("latest".to_string()));
        let url = format!("{}/{}", exchange.api_url.as_str(), self.infura_id);
        match geth::rpc(&url, "eth_call", geth::ParamTypes::Infura(params)) {
            Ok(result) => Ok(result.part),
            Err(e) => Err(e),
        }
    }
}

pub fn read_pairs(filename: &str) -> PairList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let pairs = serde_yaml::from_str::<HashMap<String, PairDetail>>(&yaml).unwrap();
    PairList { pairs: pairs }
}

pub fn read_abi(filename: &str) -> Contract {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let abi = serde_yaml::from_str::<Vec<AbiCall>>(&yaml).unwrap();
    Contract { abi: abi }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AbiInput {
    #[serde(default)]
    indexed: bool,
    name: String,
    r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AbiCall {
    r#type: String,
    name: Option<String>,
    inputs: Vec<AbiInput>,
}

pub struct Contract {
    abi: Vec<AbiCall>,
}

impl Contract {
    fn call(&self, fname: &str) -> Option<Vec<u8>> {
        let call_opt = self.abi.iter().find(|r#fn| {
            if let Some(name) = &r#fn.name {
                name == fname
            } else {
                false
            }
        });
        match call_opt {
            Some(call) => {
                if let Some(name) = &call.name {
                    Some(eth::hash_abi_sig(&name).to_vec())
                } else {
                    None
                }
            }
            None => None,
        }
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
        let pub_addr = format!("0x{}", eth::privkey_to_addr(privkey));
        let pair = self.pairs.get(&market.base.symbol, &market.quote.symbol);
        let offer_cost = offer.cost(*askbid);
        let qty_int = exchange::quantity_in_base_units(
            offer.base_qty,
            pair.base_precision,
            pair.base_precision,
        );
        let qty_str = qty_int.to_str_radix(10);
        let cost_int = exchange::quantity_in_base_units(
            offer_cost,
            pair.quote_precision,
            pair.quote_precision,
        );
        let cost_str = cost_int.to_str_radix(10);
        println!(
            "qty {} {} cost {} {}",
            offer.base_qty, qty_int, offer_cost, cost_int
        );

        let base_token = &self.tokens.get(&pair.base).address;
        let quote_token = &self.tokens.get(&pair.quote).address;
        let min_sell = match self.min_sell(quote_token, exchange).unwrap() {
            geth::ResultTypes::Result(r) => {
                let units = u64::from_str_radix(&r.result[2..], 16).unwrap();
                let qty = exchange::units_to_quantity(units, pair.quote_precision);
                println!(
                    "Min-Sell {} ^{} {} = {}",
                    &pair.quote, pair.quote_precision, units, qty
                );
                qty
            }
            geth::ResultTypes::Error(e) => {
                println!("Err {:?}", e.error.message);
                0.0
            }
        };
        if offer_cost < min_sell {
            let order_error = exchange::OrderError {
                msg: format!(
                    "minimum quantity of {} not met with {}",
                    min_sell, offer_cost
                ),
                code: 14 as i32,
            };
            println!("ERR: {}", order_error);
            return Err(Box::new(order_error));
        }

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
            let params = (sheet.address.clone(), "latest".to_string());
            let url = format!("{}/{}", exchange.api_url.as_str(), self.infura_id);
            let result = geth::rpc(
                &url,
                "eth_getTransactionCount",
                geth::ParamTypes::InfuraSingle(params),
            )
            .unwrap();
            let nonce = match result.part {
                geth::ResultTypes::Result(r) => u32::from_str_radix(&r.result[2..], 16).unwrap(),
                geth::ResultTypes::Error(e) => {
                    println!("Err {:?}", e.error.message);
                    0
                }
            };
            println!("TX Count/next nonce {}", nonce);
            let gas_prices = geth::ethgasstation();
            let gas_price = (gas_prices.fast as f64 * 100_000_000u64 as f64) as u64;
            let gas_price_gwei = gas_price / 1_000_000_000u64;
            println!(
                "gas prices {:?} final price {}gwei",
                gas_prices, gas_price_gwei
            );

            let mut contract_addra = [0u8; 20];
            let contract_addr = exchange.contract_address.clone();
            contract_addra.copy_from_slice(&eth::dehex(&contract_addr)[..]);
            let tx = ethereum_tx_sign::RawTransaction {
                nonce: ethereum_types::U256::from(nonce),
                to: Some(ethereum_types::H160::from(contract_addra)),
                value: ethereum_types::U256::zero(),
                gas_price: ethereum_types::U256::from(gas_price),
                gas: ethereum_types::U256::from(310240),
                data: eth_data(&self.contract, &sheet),
            };
            println!(
                "nonce {} gas_price {} gas_limit {}",
                tx.nonce, tx.gas_price, tx.gas
            );
            let private_key = ethereum_types::H256::from_slice(&eth::dehex(private_key));
            let rlp_bytes = tx.sign(&private_key, &eth::ETH_CHAIN_MAINNET);
            let params = (eth::hex(&rlp_bytes),);

            let url = format!("{}/{}", exchange.api_url.as_str(), self.infura_id);
            let result = geth::rpc(
                &url,
                "eth_sendRawTransaction",
                geth::ParamTypes::Single(params),
            )
            .unwrap();
            match result.part {
                geth::ResultTypes::Error(e) => println!("RPC ERR {:?}", e),
                geth::ResultTypes::Result(r) => println!("{:?}", r),
            };

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
        _privkey: &str,
        _exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceList {
        collections::HashMap::new()
    }
}

pub fn eth_data(contract: &Contract, sheet: &OrderSheet) -> Vec<u8> {
    contract.call("placeholder");
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("offer(uint256,address,uint256,address,uint256)").to_vec();
    call.append(&mut func);
    let mut p1 = hex::decode(eth::encode_uint256(&sheet.amount_sell)).unwrap();
    call.append(&mut p1);
    let mut p2 = hex::decode(eth::encode_addr2(&sheet.token_sell)).unwrap();
    call.append(&mut p2);
    let mut p3 = hex::decode(eth::encode_uint256(&sheet.amount_buy)).unwrap();
    call.append(&mut p3);
    let mut p4 = hex::decode(eth::encode_addr2(&sheet.token_buy)).unwrap();
    call.append(&mut p4);
    let mut p5 = hex::decode(eth::encode_uint256("0")).unwrap();
    call.append(&mut p5); // position
    call
}

pub fn get_min_sell_data(addr: &str) -> String {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("getMinSell(address)").to_vec();
    call.append(&mut func);
    let mut p1 = hex::decode(eth::encode_addr2(addr)).unwrap();
    call.append(&mut p1);
    format!("0x{}", hex::encode(call))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_abi_data() {
        let _api_str = "offer(uint256, address, uint256, address, uint256)";
        let contract = read_abi("notes/oasis-abi.json");
        let sheet = OrderSheet {
            address: "0xab".to_string(),
            token_buy: "0x12".to_string(),
            amount_buy: "1".to_string(),
            token_sell: "0x34".to_string(),
            amount_sell: "2".to_string(),
        };
        let _abi_hex = eth_data(&contract, &sheet);
    }
}
