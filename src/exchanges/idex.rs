use crate::config;
use crate::eth;
use crate::exchange;
use crate::types;
use reqwest::header;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSheet {
    address: String,
    token_buy: String,
    amount_buy: String,
    token_sell: String,
    amount_sell: String,
    nonce: String,
    expires: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheetSigned {
    #[serde(flatten)]
    order_sheet: OrderSheet,
    v: u8,
    r: String,
    s: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    #[serde(flatten)]
    balances: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceResponse {
    nonce: u128, //docs wrong
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenDetail {
    name: String,
    address: String,
    decimals: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenList {
    #[serde(flatten)]
    tokens: HashMap<String, TokenDetail>,
}

impl TokenList {
    pub fn read_tokens(filename: &str) -> TokenList {
        let file_ok = fs::read_to_string(filename);
        let yaml = file_ok.unwrap();
        let tokens = serde_yaml::from_str(&yaml).unwrap();
        TokenList { tokens: tokens }
    }

    pub fn get(&self, symbol2: &str) -> &TokenDetail {
        self.tokens
            .iter()
            .find(|(symbol, detail)| *symbol == symbol2)
            .unwrap()
            .1
    }
}

pub struct Idex {
    settings: config::ExchangeSettings,
    client: reqwest::blocking::Client,
    tokens: TokenList,
}

impl Idex {
    pub fn new(settings: config::ExchangeSettings, api_key: &str) -> Idex {
        let client = Idex::build_http_client(api_key).unwrap();
        let tokens = TokenList::read_tokens("notes/idex-tokens.json");
        println!("idex loaded {} tokens", tokens.tokens.len());
        Idex {
            settings: settings,
            client: client,
            tokens: tokens,
        }
    }

    pub fn build_http_client(api_key: &str) -> reqwest::Result<reqwest::blocking::Client> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "API-Key",
            header::HeaderValue::from_str(api_key).unwrap(), //boom
        );
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()
    }
}

impl exchange::Api for Idex {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &exchange::Market,
        offer: &types::Offer,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        let address = eth::privkey_to_addr(privkey);
        let base_token = self.tokens.get(&market.base.symbol);
        let quote_token = self.tokens.get(&market.quote.symbol);
        let base_qty = exchange::quantity_in_base_units(
            offer.base_qty,
            base_token.decimals,
            base_token.decimals,
        );
        let quote_qty = exchange::quantity_in_base_units(
            offer.cost(*askbid),
            base_token.decimals,
            base_token.decimals,
        );
        let url = format!(
            "{}/returnNextNonce?address=0x{}",
            exchange.api_url.as_str(),
            address
        );
        let resp = self.client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        println!("{} {}", url, status);
        let nonce_response = resp.json::<NonceResponse>().unwrap();
        Ok(exchange::OrderSheet::Idex(OrderSheet {
            token_buy: base_token.address.clone(), //market.base_contract.clone(),
            amount_buy: base_qty.to_str_radix(10),
            token_sell: quote_token.address.clone(), //market.quote_contract.clone(),
            amount_sell: quote_qty.to_str_radix(10),
            address: format!("0x{}", address),
            nonce: nonce_response.nonce.to_string(),
            expires: 10000,
        }))
    }

    fn submit(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let exchange::OrderSheet::Idex(order_sheet) = sheet {
            let privbytes = &hex::decode(privkey).unwrap();
            let secret_key = SecretKey::from_slice(privbytes).unwrap();
            let order_hash_bytes = order_params_hash(&order_sheet, &exchange.contract_address);
            let order_hash = eth::ethsign_hash_msg(&order_hash_bytes[..].to_vec());
            let (v, r, s) = eth::sign_bytes_vrs(&order_hash, &secret_key);
            let signed = OrderSheetSigned {
                order_sheet: order_sheet,
                v: v,
                r: eth::hex(&r),
                s: eth::hex(&s),
            };
            let url = format!("{}/order", exchange.api_url.as_str(),);
            println!("{:?}", signed);
            let resp = self.client.post(url.as_str()).json(&signed).send().unwrap();
            let status = resp.status();
            let json = resp.text();
            println!("{} {} {:?}", url, status, json);
        };
        Ok(())
    }

    fn balances<'a>(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceList {
        let url = format!(
            "{}/returnBalances?address=0x{}",
            exchange.api_url.as_str(),
            public_addr
        );
        let resp = self.client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        let response = resp.json::<BalanceResponse>().unwrap();
        response
            .balances
            .iter()
            .map(|(symbol, strval)| {
                let f64 = strval.parse::<f64>().unwrap();
                (symbol.clone(), f64)
            })
            .collect()
    }
}

pub fn order_params_hash(order: &OrderSheet, contract_address: &str) -> [u8; 32] {
    let expires = order.expires.to_string();
    let mut parts: Vec<Vec<u8>> = vec![
        encode_addr(contract_address),
        encode_addr(&order.token_buy),
        encode_uint256(&order.amount_buy),
        encode_addr(&order.token_sell),
        encode_uint256(&order.amount_sell),
        encode_uint256(&expires),
        encode_uint256(&order.nonce),
        encode_addr(&order.address),
    ];
    let hash_hex = parts.iter_mut().fold(Vec::<u8>::new(), |mut memo, part| {
        memo.append(part);
        memo
    });
    let hashes = hex::decode(&hash_hex).unwrap();
    eth::hash_msg(&hashes)
}

pub fn encode_addr(zstr: &str) -> Vec<u8> {
    // 160bits/20bytes
    let hexletters = zstr[2..].to_lowercase();
    //rlp::encode(&hexletters) //.as_bytes().to_vec()
    hexletters.as_bytes().to_vec() // do nothing?
}

pub fn encode_uint256(numstr: &str) -> Vec<u8> {
    // 256bits/32bytes
    let num = numstr.parse::<u128>().unwrap();
    let number = format!("{:x}", num);
    left_pad_zero(number.as_bytes().to_vec(), 64)
}

pub fn left_pad_zero(bytes: Vec<u8>, width: u8) -> Vec<u8> {
    let padding_char = '0' as u8;
    let mut padded = Vec::<u8>::new();
    let left = (width as usize) - bytes.len();
    for x in 0..left {
        padded.push(padding_char)
    }
    padded.append(&mut bytes.clone());
    padded
}

#[cfg(test)]
mod tests {
    use super::*;

    static PRIVKEY_DDEX3: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";

    #[test]
    fn test_encode_addr() {
        let idex_contract = "0x2a0c0dbecc7e4d658f48e01e3fa353f44050c208";
        let idex_encoded = hex::decode(encode_addr(idex_contract)).unwrap();
        let hash = eth::hash_msg(&idex_encoded);
        let good_hash = "0x9f13f88230a70de90ed5fa41ba35a5fb78bc55d11cc9406f17d314fb67047ac7";
        assert_eq!(hex::encode(hash), good_hash[2..]);
    }

    #[test]
    fn test_encode_addr2() {
        //web3.js docs
        let idex_contract = "0x407D73d8a49eeb85D32Cf465507dd71d507100c1";
        let idex_encoded = hex::decode(encode_addr(idex_contract)).unwrap();
        let hash = eth::hash_msg(&idex_encoded);
        let good_hash = "0x4e8ebbefa452077428f93c9520d3edd60594ff452a29ac7d2ccc11d47f3ab95b";
        assert_eq!(hex::encode(hash), good_hash[2..]);
    }

    #[test]
    fn test_left_pad_zero() {
        let bytes = vec![1, 2, 3];
        let padded = left_pad_zero(bytes, 4);
        let good = vec!['0' as u8, 1, 2, 3];
        assert_eq!(good, padded);

        let bytes = vec![1, 2, 3, 4];
        let padded = left_pad_zero(bytes, 4);
        let good = vec![1, 2, 3, 4];
        assert_eq!(good, padded);
    }

    #[test]
    fn test_encode_uint256() {
        let number = "1";
        let idex_encoded = hex::decode(encode_uint256(number)).unwrap();
        let hash = eth::hash_msg(&idex_encoded);
        let good_hash = "0xb10e2d527612073b26eecdfd717e6a320cf44b4afac2b0732d9fcbe2b7fa0cf6";
        assert_eq!(hex::encode(hash), good_hash[2..]);
    }

    #[test]
    fn test_order_params_hash() {
        let address = eth::privkey_to_addr(PRIVKEY_DDEX3);

        /*
          "tokenBuy": "0x0000000000000000000000000000000000000000",
          "amountBuy": "150000000000000000",
          "tokenSell": "0xcdcfc0f66c522fd086a1b725ea3c0eeb9f9e8814",
          "amountSell": "1000000000000000000000",
          "address": "0xed6d484f5c289ec8c6b6f934ef6419230169f534",
          "nonce": 123,
          "expires": 100000,
        */
        let order_sheet = OrderSheet {
            token_buy: "0x0000000000000000000000000000000000000000".to_string(), //market.base_contract.clone(),
            amount_buy: "150000000000000000".to_string(),
            token_sell: "0xcdcfc0f66c522fd086a1b725ea3c0eeb9f9e8814".to_string(), //market.quote_contract.clone(),
            amount_sell: "1000000000000000000000".to_string(),
            address: format!("0x{}", address),
            nonce: 123.to_string(),
            expires: 100000,
        };
        let idex_contract = "0x2a0c0dbecc7e4d658f48e01e3fa353f44050c208";
        let order_hash_bytes = order_params_hash(&order_sheet, idex_contract);
        let good_hash = "0x385777b82d67f8368848ccd56f6ad04159bb6fc1075ae06910abb597c5a7c6a0";
        assert_eq!(good_hash[2..], hex::encode(order_hash_bytes));
    }

    #[test]
    fn test_order_params_sign() {
        let privbytes = &hex::decode(PRIVKEY_DDEX3).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();

        let order_hash_str = "0x385777b82d67f8368848ccd56f6ad04159bb6fc1075ae06910abb597c5a7c6a0";
        let order_params_hash = hex::decode(&order_hash_str[2..]).unwrap();
        let order_hash = eth::ethsign_hash_msg(&order_params_hash[..].to_vec());
        let (v, r, s) = eth::sign_bytes_vrs(&order_hash, &secret_key);

        let good_r = "0x860874c6d650c646389e3a7fbcd835665e546cbafa9831438d3a71535c19c50f";
        let good_s = "0x18205ecf4a6927e8653828c5508c3676f634c74051d9ef4f9216dbef43594a25";

        assert_eq!(hex::encode(r), good_r[2..]);
        assert_eq!(hex::encode(s), good_s[2..]);
    }
}
