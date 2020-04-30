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
    pub name: String,
    pub address: String,
    pub decimals: i32,
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
        let detail = self
            .tokens
            .iter()
            .find(|(symbol, _detail)| *symbol == symbol2)
            .unwrap()
            .1;
        println!(
            "idex lookup {} ^{} {}",
            symbol2, detail.decimals, detail.address
        );
        detail
    }

    pub fn by_addr(&self, addr: &str) -> &TokenDetail {
        let detail = self
            .tokens
            .iter()
            .find(|(symbol, _detail)| _detail.address == addr)
            .unwrap();
        println!(
            "idex by_addr {} ^{} {}",
            detail.0, detail.1.decimals, detail.1.address
        );
        detail.1
    }
}

#[allow(dead_code)]
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
        eth::encode_addr(contract_address),
        eth::encode_addr(&order.token_buy),
        eth::encode_uint256(&order.amount_buy),
        eth::encode_addr(&order.token_sell),
        eth::encode_uint256(&order.amount_sell),
        eth::encode_uint256(&expires),
        eth::encode_uint256(&order.nonce),
        eth::encode_addr(&order.address),
    ];
    let hash_hex = parts.iter_mut().fold(Vec::<u8>::new(), |mut memo, part| {
        memo.append(part);
        memo
    });
    let hashes = hex::decode(&hash_hex).unwrap();
    eth::hash_msg(&hashes)
}

#[cfg(test)]
mod tests {
    use super::*;

    static PRIVKEY_DDEX3: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";

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
        let (_v, r, s) = eth::sign_bytes_vrs(&order_hash, &secret_key);

        let good_r = "0x860874c6d650c646389e3a7fbcd835665e546cbafa9831438d3a71535c19c50f";
        let good_s = "0x18205ecf4a6927e8653828c5508c3676f634c74051d9ef4f9216dbef43594a25";

        assert_eq!(hex::encode(r), good_r[2..]);
        assert_eq!(hex::encode(s), good_s[2..]);
    }
}
