use crate::config;
use crate::eth;
use crate::exchange;
use crate::time;
use crate::types;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use num_traits::cast::FromPrimitive;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

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

impl From<&types::AskBid> for BuySell {
    fn from(askbid: &types::AskBid) -> Self {
        match askbid {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        }
    }
}

impl Into<exchange::BuySell> for BuySell {
    fn into(self) -> exchange::BuySell {
        match self {
            BuySell::Buy => exchange::BuySell::Buy,
            BuySell::Sell => exchange::BuySell::Sell,
        }
    }
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
pub enum OrderStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "completed")]
    Completed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    id: String,
    created_at: String,
    order_status: OrderStatus,
    side: BuySell,
    price: String,
    quantity: String,
    pair: String,
}

impl Order {
    fn into_exg(self, base_token: &TokenDetail, quote_token: &TokenDetail) -> exchange::Order {
        exchange::Order {
            id: self.id,
            side: self.side.into(),
            state: exchange::OrderState::Cancelled,
            market: self.pair,
            base_qty: units_to_amount(&self.quantity, base_token),
            quote: units_to_amount(&self.price, quote_token),
            create_date: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
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
}

impl exchange::Api for Switcheo {
    fn setup(&mut self) {}

    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &exchange::Market,
        offer: &types::Offer,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        println!(
            "={:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );

        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
        let market_pair = make_market_pair(market);
        let now_millis = time::now();
        let base_token_detail = self.tokens.get(&market.base).unwrap();
        let quote_token_detail = self.tokens.get(&market.quote).unwrap();
        let market_detail = self.pairs.get(&market_pair).unwrap();

        let sheet = OrderSheet {
            blockchain: "eth".to_string(),
            contract_hash: exchange.contract_address.to_string(),
            order_type: "limit".to_string(),
            pair: market_pair,
            price: amount_to_units(offer.quote, market_detail.precision, quote_token_detail),
            quantity: amount_to_units(
                offer.base_qty,
                base_token_detail.precision,
                base_token_detail,
            ),
            side: askbid.into(),
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
        println!("switcheo build {}", url);
        println!("{}", serde_json::to_string(&sheet_sign.sheet).unwrap());
        let client = reqwest::blocking::Client::new();
        let resp = client.post(url.as_str()).json(&sheet_sign).send().unwrap();
        let status = resp.status();
        println!("switcheo build result {:#?} {}", status, resp.url());
        if status.is_success() {
            let build_success = resp.json::<Order>().unwrap();
            println!("{:?}", build_success);
            Ok(exchange::OrderSheet::Switcheo(sheet_sign))
        } else {
            let build_err = resp.json::<ResponseError>().unwrap();
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
    ) -> HashMap<String, f64> {
        let url = format!(
            "{}/balances?addresses=0x{}&contract_hashes={}",
            exchange.api_url.as_str(),
            public_addr,
            exchange.contract_address
        );
        let client = reqwest::blocking::Client::new();
        let resp = client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        let balances = resp.json::<BalanceResponse>().unwrap();
        balances
            .confirmed
            .iter()
            .map(|(symbol, units)| {
                match self.tokens.get(&types::Ticker {
                    symbol: symbol.to_string(),
                }) {
                    Some(token) => {
                        let f_bal = units_to_amount(units, token);
                        (symbol.clone(), f_bal)
                    }
                    None => (format!("conversion-err {} {}", symbol, units), 0.0),
                }
            })
            .collect()
    }

    fn withdrawl(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: types::Ticker,
    ) {
        let url = format!("{}/deposits", exchange.api_url.as_str());
    }

    fn open_orders(
        &self,
        account: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<exchange::Order> {
        let url = format!(
            "{}/orders?address=0x{}&contract_hashes={}",
            exchange.api_url.as_str(),
            account,
            exchange.contract_address
        );
        println!("{}", url);
        let client = reqwest::blocking::Client::new();
        let resp = client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        if status.is_success() {
            let orders = resp.json::<Vec<Order>>().unwrap();
            println!("switcheo raw orders {:?}", orders);
            let shortlist = Vec::<exchange::Order>::new();
            orders.into_iter().fold(shortlist, |mut m, o| {
                let (base_name, quote_name) = split_market_pair(&o.pair);
                match self.tokens.get(&types::Ticker {
                    symbol: base_name.to_string(),
                }) {
                    Some(base_token) => {
                        match self.tokens.get(&types::Ticker {
                            symbol: quote_name.to_string(),
                        }) {
                            Some(quote_token) => m.push(o.into_exg(base_token, quote_token)),
                            None => (),
                        }
                    }
                    None => (),
                }
                m
            })
        } else {
            let build_err = resp.json::<ResponseError>().unwrap();
            println!("{:?}", build_err);
            vec![] // bad
        }
    }
}

pub fn make_market_pair(market: &exchange::Market) -> String {
    format!("{}_{}", market.base.symbol, market.quote.symbol)
}

pub fn split_market_pair(pair: &str) -> (String, String) {
    let parts: Vec<&str> = pair.split("_").collect();
    (parts[0].to_string(), parts[1].to_string())
}

pub fn sign<'a>(json: &String, secret_key: &SecretKey) -> String {
    let msg_hash = eth::ethsign_hash_msg(&json.as_bytes().to_vec());
    let sig_bytes = eth::sign_bytes(&msg_hash, &secret_key);
    format!("0x{}", hex::encode(sig_bytes.to_vec()))
}

pub fn amount_to_units(amount: f64, precision: i32, token: &TokenDetail) -> String {
    let qty_int = quantity_in_base_units(amount, precision, token.decimals);
    let qty_str = qty_int.to_str_radix(10);
    println!(
        "{}^{} {}^{} => \"{}\"",
        amount, precision, token.symbol, token.decimals, qty_str
    );
    qty_str
}

pub fn units_to_amount(units: &str, token: &TokenDetail) -> f64 {
    let unts = units.parse::<u128>().unwrap();
    let power = 10_u128.pow(token.decimals as u32);
    unts as f64 / power as f64
}

pub fn quantity_in_base_units(qty: f64, prec: i32, scale: i32) -> BigInt {
    let big_dec = BigDecimal::from_f64(qty)
        .unwrap()
        .with_scale(prec as i64) // truncates
        .with_scale(scale as i64);
    let (qty_int, exp) = big_dec.into_bigint_and_exponent();
    qty_int
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
        let token = TokenDetail {
            symbol: "BAT".to_string(),
            name: "BAT".to_string(),
            r#type: "wut".to_string(),
            hash: "abc".to_string(),
            decimals: 18,
            transfer_decimals: 18,
            precision: 2,
            minimum_quantity: "0".to_string(),
            trading_active: true,
            is_stablecoin: false,
            stablecoin_type: None,
        };
        let units = amount_to_units(2.3, 2, &token);
        assert_eq!(units, "2300000000000000000") // float sigma fun
    }

    #[test]
    fn test_quantity_in_base_units() {
        let unit_q = quantity_in_base_units(1.1234, 2, 18);
        assert_eq!(unit_q, 1120000000000000000_u64.into());
        let unit_q = quantity_in_base_units(100.1234, 2, 18);
        assert_eq!(unit_q, 100120000000000000000_u128.into());
        let unit_q = quantity_in_base_units(0.234, 8, 8);
        assert_eq!(unit_q, 23400000.into());
        let unit_q = quantity_in_base_units(2.3, 1, 2);
        assert_eq!(unit_q, 230.into());
    }

    #[test]
    fn test_units_to_amount() {
        let token = TokenDetail {
            symbol: "BAT".to_string(),
            name: "BAT".to_string(),
            r#type: "wut".to_string(),
            hash: "abc".to_string(),
            decimals: 8,
            transfer_decimals: 8,
            precision: 2,
            minimum_quantity: "0".to_string(),
            trading_active: true,
            is_stablecoin: false,
            stablecoin_type: None,
        };
        let amt = units_to_amount("123456789", &token);
        assert_eq!(amt, 1.23456789)
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
