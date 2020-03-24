use crate::config;
use crate::eth;
use crate::exchange;
use crate::types;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Pair {
    name: String,
    precision: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PairList {
    pairs: Vec<Pair>,
}

impl PairList {
    pub fn get(&self, market: &str) -> Option<&Pair> {
        let mut result: Option<&Pair> = None;
        for pair in &self.pairs {
            if pair.name == market {
                result = Some(&pair)
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }
}

pub fn read_pairs(filename: &str) -> PairList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let pairs = serde_yaml::from_str::<Vec<Pair>>(&yaml).unwrap();
    PairList { pairs: pairs }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenDetail {
    symbol: String,
    name: String,
    r#type: String,
    hash: String,
    decimals: i32,
    transfer_decimals: i32,
    precision: i32,
    minimum_quantity: String,
    trading_active: bool,
    is_stablecoin: bool,
    stablecoin_type: Option<String>,
}

pub struct TokenList {
    tokens: HashMap<String, TokenDetail>,
}

pub fn read_tokens(filename: &str) -> TokenList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let tokens = serde_yaml::from_str(&yaml).unwrap();
    TokenList { tokens: tokens }
}

impl TokenList {
    pub fn get(&self, ticker: &types::Ticker) -> Option<&TokenDetail> {
        self.tokens.get(&ticker.symbol)
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }
}

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
    address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    blockchain: String,
    contract_hash: String,
    order_type: String,
    pair: String,
    price: String,    // market-specified precision
    quantity: String, // integer unit quantity
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

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildSuccess {}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildError {
    error: String,
    #[serde(default)]
    error_message: String,
    #[serde(default)]
    error_code: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    confirming: HashMap<String, BalanceConfirming>,
    confirmed: HashMap<String, String>,
    locked: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceConfirming {
    event_type: String,
    asset_id: String,
    amount: u32,
    transaction_hash: (),
    created_at: String,
}

pub struct Switcheo {
    tokens: TokenList,
    pairs: PairList,
}

impl Switcheo {
    pub fn new() -> Switcheo {
        let tokens = read_tokens("notes/switcheo-tokens.json");
        let pairs = read_pairs("notes/switcheo-pairs.json");
        println!(
            "switcheo loaded {} tokens and {} pairs",
            tokens.len(),
            pairs.len()
        );
        Switcheo {
            tokens: tokens,
            pairs: pairs,
        }
    }

    pub fn amount_to_units(&self, amount: f64, precision: i32, token: &TokenDetail) -> String {
        let units = quantity_in_base_units(amount, precision as u32);
        let remaining: usize = (token.decimals - precision) as usize;
        let qty_str = format!("{}{}", units, "0".repeat(remaining));
        println!(
            "{}^{} {}^{} => \"{}\"",
            amount, precision, token.symbol, token.decimals, qty_str
        );
        qty_str
    }
}

impl exchange::Api for Switcheo {
    fn setup(&mut self) {}

    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &types::Market,
        offer: &types::Offer,
        proxy: Option<String>,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        println!(
            "={:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );

        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
        let side = match askbid {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        };

        let market_pair = make_market_pair(market.swapped, &market.base, &market.quote);

        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let base_token_detail = self.tokens.get(&market.base).unwrap();
        let quote_token_detail = self.tokens.get(&market.quote).unwrap();
        let market_detail = self.pairs.get(&market_pair).unwrap();

        let sheet = OrderSheet {
            blockchain: "eth".to_string(),
            contract_hash: exchange.contract_address.to_string(),
            order_type: "limit".to_string(),
            pair: market_pair,
            price: self.amount_to_units(offer.quote, market_detail.precision, quote_token_detail),
            quantity: self.amount_to_units(
                offer.base_qty,
                base_token_detail.precision,
                base_token_detail,
            ),
            side: side,
            timestamp: now_millis,
            use_native_tokens: false,
        };
        let sign_json = serde_json::to_string(&sheet).unwrap();
        let signature = sign(&sign_json, &secret_key);
        let address = format!("0x{}", eth::privkey_to_addr(privkey));
        println!("{:#?}", sheet);
        let sheet_sign = OrderSheetSign {
            address: address,
            sheet: sheet,
            signature: signature,
        };

        let url = format!("{}/orders", exchange.api_url.as_str());
        println!("switcheo limit order build {}", url);
        println!("{}", serde_json::to_string(&sheet_sign.sheet).unwrap());
        let client = reqwest::blocking::Client::new();
        let resp = client.post(url.as_str()).json(&sheet_sign).send().unwrap();
        let status = resp.status();
        println!("switcheo result {:#?} {}", status, resp.url());
        //let text = resp.text().unwrap();
        //println!("{}", text);
        if status.is_success() {
            Ok(exchange::OrderSheet::Switcheo(sheet_sign))
        } else {
            let build_err = resp.json::<BuildError>().unwrap();
            let order_error = exchange::OrderError {
                msg: build_err.error,
                code: build_err.error_code as i32,
            };
            println!("ERR: {}", order_error);
            Err(Box::new(order_error))
        }
    }

    fn submit(
        &self,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn balances<'a>(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<(&str, f64)> {
        let url = format!(
            "{}/balances?addresses={}&contract_hashes={}",
            exchange.api_url.as_str(),
            public_addr,
            exchange.contract_address
        );
        let client = reqwest::blocking::Client::new();
        let resp = client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        let balances = resp.json::<BalanceResponse>().unwrap();
        println!("{} {:#?}", status, balances);
        //  "confirmed": {"GAS": "47320000000.0",
        vec![]
    }

    fn deposit(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: types::Ticker,
    ) {
        let url = format!("{}/deposits", exchange.api_url.as_str());
    }
}

pub fn make_market_pair(swapped: bool, base: &types::Ticker, quote: &types::Ticker) -> String {
    match swapped {
        true => format!("{}_{}", quote.symbol, base.symbol),
        false => format!("{}_{}", base.symbol, quote.symbol),
    }
}

pub fn sign<'a>(json: &String, secret_key: &SecretKey) -> String {
    let msg_hash = eth::ethsign_hash_msg(&json.as_bytes().to_vec());
    let sig_bytes = eth::sign_bytes(&msg_hash, &secret_key);
    format!("0x{}", hex::encode(sig_bytes.to_vec()))
}

pub fn quantity_in_base_units(qty: f64, exp: u32) -> u64 {
    (qty * 10_f64.powi(exp as i32)) as u64
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

    #[test]
    fn test_amount_to_units() {
        let switcheo = Switcheo::new();
        let ticker = types::Ticker {
            symbol: "ETH".to_string(),
        };
        let units = switcheo.amount_to_units(2.3, &ticker);
        assert_eq!(units, "2300000000000000000") // float sigma fun
    }

    #[test]
    fn test_quantity_in_base_units() {
        let unit_q = quantity_in_base_units(1.0, 18);
        assert_eq!(unit_q, 1000000000000000000);
        let unit_q = quantity_in_base_units(1.234, 8);
        assert_eq!(unit_q, 123400000);
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
