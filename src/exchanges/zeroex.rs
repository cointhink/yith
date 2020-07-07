use crate::config;
use crate::eth;
use crate::exchange;
use crate::geth;
use crate::http;
use crate::time;
use crate::types;
use chrono;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    r#type: BuySell,
    quantity: String,
    price: String,
    expiration: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketOrders {
    orders: Vec<OrderForm>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderForm {
    chain_id: i64,
    maker_address: String,
    signature: String,
    sender_address: String,
    exchange_address: String,
    taker_address: String,
    maker_fee: String,
    taker_fee: String,
    maker_fee_asset_data: String,
    taker_fee_asset_data: String,
    maker_asset_amount: String,
    taker_asset_amount: String,
    maker_asset_data: String,
    taker_asset_data: String,
    salt: String,
    fee_recipient_address: String,
    expiration_time_seconds: String,
}

impl OrderForm {
    pub fn taker_qty(&self, decimals: u32) -> f64 {
        let units = u128::from_str_radix(&self.taker_asset_amount, 10).unwrap();
        exchange::units_to_quantity(units, decimals as i32)
    }
    pub fn maker_qty(&self, decimals: u32) -> f64 {
        let units = u128::from_str_radix(&self.maker_asset_amount, 10).unwrap();
        exchange::units_to_quantity(units, decimals as i32)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    order_hash: String,
    r#type: String,
    state: OrderState,
    base_token_address: String,
    quote_token_address: String,
    remaining_base_token_amount: String,
    remaining_quote_token_amount: String,
    price: String,
    created_date: String,
}
/*    "orderHash": "0x0cfaaa4e4f0a4409573b711ace266c2a10f7b025d0ec96567738353f32bd4eb1",
    "type": "BID",
    "state": "UNFUNDED",
    "baseTokenAddress": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "quoteTokenAddress": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
    "remainingBaseTokenAmount": "0.037784",
    "remainingQuoteTokenAmount": "10.03883096",
    "price": "265.69",
    "createdDate": "2020-03-18 17:41:39",
*/

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum OrderState {
    #[serde(rename = "OPEN")]
    Open,
    #[serde(rename = "FILLED")]
    Filled,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "EXPIRED")]
    Expired,
    #[serde(rename = "UNFUNDED")]
    Unfunded,
}

impl Into<exchange::OrderState> for OrderState {
    fn into(self) -> exchange::OrderState {
        match self {
            OrderState::Open => exchange::OrderState::Open,
            OrderState::Filled => exchange::OrderState::Filled,
            OrderState::Cancelled => exchange::OrderState::Cancelled,
            OrderState::Expired => exchange::OrderState::Expired,
            OrderState::Unfunded => exchange::OrderState::Unfunded,
        }
    }
}

impl Order {
    pub fn to_exchange_order(&self) -> exchange::Order {
        let side = match self.r#type.as_str() {
            "ASK" => Ok(exchange::BuySell::Sell),
            "BID" => Ok(exchange::BuySell::Buy),
            _ => Err(self.r#type.clone()),
        }
        .unwrap();
        let date =
            chrono::NaiveDateTime::parse_from_str(self.created_date.as_str(), "%Y-%m-%d %H:%M:%S")
                .unwrap();
        let market = format!(
            "{}-{}",
            &self.base_token_address[0..6],
            &self.quote_token_address[0..6]
        );
        exchange::Order {
            id: self.order_hash.clone(),
            side: side,
            state: self.state.into(),
            market: market,
            base_qty: f64::from_str(self.remaining_base_token_amount.as_str()).unwrap(),
            quote: f64::from_str(self.price.as_str()).unwrap(),
            create_date: date.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    status: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum BuySell {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

#[allow(dead_code)]
pub enum SignatureType {
    Illegal = 0x00,
    Invalid = 0x01,
    Eip712 = 0x02,
    EthSign = 0x03,
    Wallet = 0x04,
    Validator = 0x05,
    PreSigned = 0x06,
    EIP1271Wallet = 0x07,
}

pub fn read_tokens(filename: &str) -> TokenList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let tokens = serde_yaml::from_str(&yaml).unwrap();
    TokenList { tokens: tokens }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenList {
    tokens: Vec<Token>,
}

impl TokenList {
    pub fn by_addr(&self, addr: &str) -> &Token {
        self.tokens
            .iter()
            .find(|detail| detail.address == addr)
            .unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    pub active: u32,
}

pub struct Zeroex {
    pub settings: config::ExchangeSettings,
    client: http::LoggingClient,
    tokens: TokenList,
}

impl Zeroex {
    pub fn new(settings: config::ExchangeSettings, geth: geth::Client) -> Zeroex {
        let tokens = read_tokens("notes/radarrelay-tokens.json");
        let client = reqwest::blocking::Client::new();
        let logging_client = http::LoggingClient::new(client);
        Zeroex {
            settings: settings,
            client: logging_client,
            tokens: tokens,
        }
    }
}

impl exchange::Api for Zeroex {
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
            "=0x build {:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );
        let qty = offer.base_qty;
        let price = offer.quote;
        let side = match askbid {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        };
        let expire_time = (time::since_epoch() + std::time::Duration::new(120 + 15, 0)).as_secs(); // 2min minimum + transittime

        let sheet = OrderSheet {
            // market order
            r#type: side,
            quantity: format!("{}", qty),
            // limit order only
            price: format!("{}", price),
            expiration: format!("{}", expire_time),
        };
        let url = format!(
            "{}/markets/{}/order/market",
            exchange.api_url.as_str(),
            market.id("-")
        );
        println!("{}", url);
        println!("{}", serde_json::to_string(&sheet).unwrap());
        let resp = self.client.post(url.as_str()).json(&sheet).send()?;
        println!("{:#?} {}", resp.status(), resp.url());
        if resp.status().is_success() {
            let mkt_orders = resp.json::<MarketOrders>().unwrap();
            let mut forms = mkt_orders
                .orders
                .into_iter()
                .fold(vec![], |mut memo, mut form| {
                    let taker_asset_addr = format!("0x{}", &form.taker_asset_data[34..74]);
                    let taker_token = self.tokens.by_addr(&taker_asset_addr);
                    let maker_asset_addr = format!("0x{}", &form.maker_asset_data[34..74]);
                    let maker_token = self.tokens.by_addr(&maker_asset_addr);
                    let taker_qty = form.taker_qty(taker_token.decimals);
                    let maker_qty = form.maker_qty(maker_token.decimals);
                    let (qty, price) = match side {
                        BuySell::Buy => (maker_qty, taker_qty / maker_qty),
                        BuySell::Sell => (taker_qty, maker_qty / taker_qty),
                    };
                    println!("offer {:?} {}@{}", side, qty, price);
                    let good_qty = eth::minimum(&vec![qty, offer.base_qty]);
                    println!("good qty {}", good_qty);
                    let better = match side {
                        BuySell::Buy => price < offer.quote,
                        BuySell::Sell => price > offer.quote,
                    };
                    println!(
                        "offer price better {} for price {}, offer quote {}",
                        better, price, offer.quote
                    );
                    if good_qty > 0.0 && better {
                        form.taker_address =
                            format!("0x{}", eth::privkey_to_addr(privkey).to_string());
                        // TODO calculate amounts
                        form.maker_asset_amount = format!("{}", 0.0);
                        form.taker_asset_amount = format!("{}", 0.0);
                        println!("{:#?}", form);
                        let privkey_bytes = &hex::decode(privkey).unwrap();
                        let order_hash = order_hash(&form);
                        form.signature = order_sign(privkey_bytes, order_hash);
                        memo.push(form);
                    }
                    memo
                });
            if forms.len() > 0 {
                Ok(exchange::OrderSheet::Zeroex(forms.remove(0)))
            } else {
                Err(Box::new(exchange::OrderError {
                    msg: "no matching offers".to_string(),
                    code: -1,
                }))
            }
        } else {
            let bodyerr = resp.json::<ErrorResponse>().unwrap();
            let order_error = exchange::OrderError {
                msg: bodyerr.error,
                code: -1,
            };
            println!("ERR: {}", order_error);
            Err(Box::new(order_error))
        }
    }

    fn submit(
        &self,
        _privkey: &str,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let exchange::OrderSheet::Zeroex(order) = sheet {
            let url = format!("{}/orders", exchange.api_url.as_str());
            println!("{}", url);
            println!("{}", serde_json::to_string(&order).unwrap());
            let resp = self.client.post(url.as_str()).json(&order).send()?;
            println!("{:#?} {}", resp.status(), resp.url());
            println!("{:#?}", resp.text());
            let order_hash = eth::hex(&order_hash(&order));
            Ok(order_hash)
        } else {
            Err(exchange::ExchangeError::build_box(format!(
                "wrong ordersheet type!"
            )))
        }
    }

    fn open_orders(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<exchange::Order> {
        let account = eth::privkey_to_addr(private_key);
        let url = format!(
            "{}/accounts/0x{}/orders",
            exchange.api_url.as_str(),
            account
        );
        println!("{}", url);
        let resp = self.client.get(url.as_str()).send().unwrap();
        //println!("{:#?} {}", resp.status(), resp.url());
        let orders = resp.json::<Vec<Order>>().unwrap();
        //println!("{:#?}", orders);
        orders
            .iter()
            .map(|native_order| native_order.to_exchange_order())
            //.filter(|order| order.state != exchange::OrderState::Cancelled)
            .collect()
    }

    fn order_status(
        &self,
        order_id: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::OrderState {
        let url = format!("{}/orders/{}", exchange.api_url.as_str(), order_id);
        let resp = self.client.get(url.as_str()).send().unwrap();
        if resp.status().is_success() {
            let order = resp.json::<Order>().unwrap();
            order.state.into()
        } else {
            exchange::OrderState::Cancelled
        }
    }
}

pub fn order_hash(form: &OrderForm) -> [u8; 32] {
    let form_tokens = order_tokens(&form);
    let form_tokens_bytes: Vec<u8> = ethabi::encode(&form_tokens);
    let form_hash = eth::hash_msg(&form_tokens_bytes);
    let exg_tokens = exchange_order_tokens(form_hash, &form.exchange_address);
    let exg_tokens_bytes: Vec<u8> = ethabi::encode(&exg_tokens);
    let eip191_header = hex::decode("1901").unwrap();
    let exg_with_header: Vec<u8> = [&eip191_header[..], &exg_tokens_bytes[..]].concat();
    eth::hash_msg(&exg_with_header)
}

pub fn order_sign(privkey_bytes: &Vec<u8>, exg_hash: [u8; 32]) -> String {
    let secret_key = SecretKey::from_slice(privkey_bytes).expect("bad secret key bytes");
    let (v, r, s) = eth::sign_bytes_vrs(&exg_hash, &secret_key);
    let form_sig_bytes = eth::sigparts_to_vrs(v, r, s);
    format!(
        "0x{}{}",
        hex::encode(&form_sig_bytes[..]),
        hex::encode(vec![SignatureType::Eip712 as u8])
    )
}

pub fn order_tokens(form: &OrderForm) -> Vec<ethabi::Token> {
    let eip712_order_schema_hash =
        hex::decode("f80322eb8376aafb64eadf8f0d7623f22130fd9491a221e902b713cb984a7534").unwrap();
    vec![
        ethabi::Token::FixedBytes(eip712_order_schema_hash),
        ethabi::Token::Address(str_to_h160(&form.maker_address[2..])),
        ethabi::Token::Address(str_to_h160(&form.taker_address[2..])),
        ethabi::Token::Address(str_to_h160(&form.fee_recipient_address[2..])),
        ethabi::Token::Address(str_to_h160(&form.sender_address[2..])),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.maker_asset_amount.parse::<u128>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.taker_asset_amount.parse::<u128>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.maker_fee.parse::<u128>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.taker_fee.parse::<u128>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.expiration_time_seconds.parse::<u128>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.salt.parse::<u128>().unwrap(),
        )),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.maker_asset_data[2..])),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.taker_asset_data[2..])),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.maker_fee_asset_data[2..])),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.taker_fee_asset_data[2..])),
    ]
}

pub fn str_to_h160(addr_str: &str) -> ethereum_types::H160 {
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hex::decode(addr_str).unwrap());
    ethereum_types::H160::from(addr)
}

pub fn hexstr_to_hashbytes(msg_str: &str) -> Vec<u8> {
    eth::hash_msg(&hex::decode(&msg_str).unwrap()).to_vec()
}

pub fn exchange_order_tokens(order_hash: [u8; 32], contract_addr: &str) -> Vec<ethabi::Token> {
    let exchange_hash = eip712_exchange_hash(contract_addr);
    vec![
        ethabi::Token::FixedBytes(exchange_hash.to_vec()),
        ethabi::Token::FixedBytes(order_hash.to_vec()),
    ]
}

pub fn eip712_exchange_hash(contract_addr: &str) -> [u8; 32] {
    let eip712_domain_schema_hash =
        hex::decode("8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f").unwrap();
    let eip712_exchange_domain_name = "0x Protocol";
    let eip712_exchange_domain_version = "3.0.0";
    let chain_id = 1;
    let tokens = vec![
        ethabi::Token::FixedBytes(eip712_domain_schema_hash),
        ethabi::Token::FixedBytes(
            eth::hash_msg(&eip712_exchange_domain_name.as_bytes().to_vec()).to_vec(),
        ),
        ethabi::Token::FixedBytes(
            eth::hash_msg(&eip712_exchange_domain_version.as_bytes().to_vec()).to_vec(),
        ),
        ethabi::Token::Uint(ethereum_types::U256::from(chain_id)),
        ethabi::Token::Address(str_to_h160(&contract_addr[2..])),
    ];
    let token_bytes = ethabi::encode(&tokens);
    eth::hash_msg(&token_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    static PRIVKEY: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";
    static CONTRACT_ADDR_V2: &str = "0x080bf510FCbF18b91105470639e9561022937712";

    fn blank_order_form() -> OrderForm {
        OrderForm {
            chain_id: 1,
            maker_address: "0x0000000000000000000000000000000000000000".to_string(),
            taker_address: "0x0000000000000000000000000000000000000000".to_string(),
            fee_recipient_address: "0x0000000000000000000000000000000000000000".to_string(),
            sender_address: "0x0000000000000000000000000000000000000000".to_string(),
            maker_asset_amount: "0".to_string(),
            taker_asset_amount: "0".to_string(),
            maker_fee: "0".to_string(),
            taker_fee: "0".to_string(),
            expiration_time_seconds: "0".to_string(),
            salt: "0".to_string(),
            exchange_address: CONTRACT_ADDR_V2.to_string(),
            maker_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            taker_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            maker_fee_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            taker_fee_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            signature: "SET".to_string(),
        }
    }

    fn docs0x_order_form() -> OrderForm {
        OrderForm {
            chain_id: 1,
            maker_address: "0x320c38912b1611a0706c0a74427f64fa5dc3598e".to_string(),
            taker_address: "0x0000000000000000000000000000000000000000".to_string(),
            fee_recipient_address: "0xc898fbee1cc94c0ff077faa5449915a506eff384".to_string(),
            sender_address: "0x0000000000000000000000000000000000000000".to_string(),
            maker_asset_amount: "47479678912656000".to_string(),
            taker_asset_amount: "43835664000000000000".to_string(),
            maker_fee: "0".to_string(),
            taker_fee: "0".to_string(),
            expiration_time_seconds: "1576876106".to_string(),
            salt: "1576832996".to_string(),
            exchange_address: "0x61935cbdd02287b511119ddb11aeb42f1593b7ef".to_string(),
            maker_asset_data: "0xf47261b0000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
            taker_asset_data: "0xf47261b00000000000000000000000004fbb350052bca5417566f188eb2ebce5b19bc964".to_string(),
            maker_fee_asset_data: "0xf47261b0000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
            taker_fee_asset_data: "0xf47261b0000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
            signature: "0x1b619623be7d177f19c2e987fa974ded5c000dd5c6dfdcb7cd17da31fe2cd94aef19ab0ffb2b29f66142a754ddb165c0d2ddc98fcb45ca7a348e4c05c77216c60103".to_string(),
        }
    }

    #[test]
    fn test_eip712_domain_sep() {
        let good_exchange_hash_v2: &str =
            "b2246130e7ae0d4b56269ccac10d3a9ac666d825bcd20ce28fea70f1f65d3de0";
        let hash = eip712_exchange_hash(CONTRACT_ADDR_V2);
        assert_eq!(hash.to_vec(), hex::decode(good_exchange_hash_v2).unwrap())
    }

    #[test]
    fn test_order_tokens() {
        let form_tokens = order_tokens(&blank_order_form());
        let form_tokens_bytes: Vec<u8> = ethabi::encode(&form_tokens);
        let good_form_tokens_bytes = "f80322eb8376aafb64eadf8f0d7623f22130fd9491a221e902b713cb984a753400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a";
        assert_eq!(
            form_tokens_bytes,
            hex::decode(good_form_tokens_bytes).unwrap()
        );
        let form_hash = eth::hash_msg(&form_tokens_bytes);
        let good_form_hash = "6272bc49657b2210a4eba2cd343aa184ed1b77c377cad3b452afa50be0f15d06";
        assert_eq!(form_hash.to_vec(), hex::decode(good_form_hash).unwrap());
    }

    #[test]
    fn test_exchange_tokens() {
        let mut form_hash = [0u8; 32];
        let good_form_hash = "6272bc49657b2210a4eba2cd343aa184ed1b77c377cad3b452afa50be0f15d06";
        form_hash.copy_from_slice(&hex::decode(good_form_hash).unwrap());
        let tokens = exchange_order_tokens(form_hash, CONTRACT_ADDR_V2);
        let exchange_tokens_bytes = ethabi::encode(&tokens);
        let eip191_header = hex::decode("1901").unwrap();
        let exg_with_header = [&eip191_header[..], &exchange_tokens_bytes[..]].concat();
        let good_exchange_tokens_bytes = "1901b2246130e7ae0d4b56269ccac10d3a9ac666d825bcd20ce28fea70f1f65d3de06272bc49657b2210a4eba2cd343aa184ed1b77c377cad3b452afa50be0f15d06";
        assert_eq!(
            exg_with_header,
            hex::decode(good_exchange_tokens_bytes).unwrap()
        );
    }

    #[test]
    fn test_exchange_hash() {
        let good_exchange_tokens_bytes = "1901b2246130e7ae0d4b56269ccac10d3a9ac666d825bcd20ce28fea70f1f65d3de06272bc49657b2210a4eba2cd343aa184ed1b77c377cad3b452afa50be0f15d06";
        let exg_tokens_bytes = hex::decode(good_exchange_tokens_bytes).unwrap();
        let exg_hash = eth::hash_msg(&exg_tokens_bytes);
        let good_exg_hash =
            hex::decode("fdc94db5a7aff3bdf03c9dc6188381c6f8fba3ead062c16a6c8b2a59427dd408")
                .unwrap();
        assert_eq!(exg_hash.to_vec(), good_exg_hash)
    }

    #[test]
    fn test_hexstr_to_hashbytes() {
        // hand "0x" prefix
        assert_eq!(
            hexstr_to_hashbytes(&"0x0000000000000000000000000000000000000000"[2..]),
            hex::decode("5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a")
                .unwrap()
        );
        // empty string case
        assert_eq!(
            hexstr_to_hashbytes(&"0x"[2..]),
            hex::decode("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
                .unwrap()
        )
    }

    #[test]
    fn test_order_hash() {
        let order_hash = order_hash(&docs0x_order_form());
        let good_hash = "0xa1e06a98da1e56bd61e81c3d9de8cbe443618ad3d77a9ba6cf0562aeaa29e1a6";
        assert_eq!(eth::hex(&order_hash), good_hash)
    }

    #[test]
    fn test_order_sign() {
        let privkey_bytes = &hex::decode(PRIVKEY).unwrap();
        let order_hash = order_hash(&blank_order_form());
        let signature = order_sign(privkey_bytes, order_hash);
        let good_sig = "0x1b4ccbff4cb18802ccaf7aaa852595170fc0443d65b1d01a10f5f01d5d65ebe42c58287ecb9cf7f62a98bdfc8931f41a157dd79e9ac5d19880f62089d9c082c79a02";
        assert_eq!(signature, good_sig)
    }
}
