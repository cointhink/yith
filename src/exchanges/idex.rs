use crate::config;
use crate::eth;
use crate::exchange;
use crate::geth;
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
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    address: String,
    amount: String,
    token: String,
    nonce: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawRequestSigned {
    #[serde(flatten)]
    withdraw_request: WithdrawRequest,
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
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    order_hash: String,
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
        // println!(
        //     "idex lookup {} ^{} {}",
        //     symbol2, detail.decimals, detail.address
        // );
        detail
    }

    pub fn by_addr(&self, addr: &str) -> &TokenDetail {
        let detail = self
            .tokens
            .iter()
            .find(|(_symbol, detail)| detail.address == addr)
            .unwrap();
        // println!(
        //     "idex by_addr {} ^{} {}",
        //     detail.0, detail.1.decimals, detail.1.address
        // );
        detail.1
    }
}

#[allow(dead_code)]
pub struct Idex {
    geth: geth::Client,
    settings: config::ExchangeSettings,
    client: reqwest::blocking::Client,
    tokens: TokenList,
}

impl Idex {
    pub fn new(settings: config::ExchangeSettings, api_key: &str, geth: geth::Client) -> Idex {
        let client = Idex::build_http_client(api_key).unwrap();
        let tokens = TokenList::read_tokens("notes/idex-tokens.json");
        println!("idex loaded {} tokens", tokens.tokens.len());
        Idex {
            geth: geth,
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
    ) -> Result<String, Box<dyn std::error::Error>> {
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
            let json = resp.text().unwrap();
            println!("{} {} {:?}", url, status, json);
            let order_response = serde_json::from_str::<OrderResponse>(&json).unwrap();
            Ok(order_response.order_hash)
        } else {
            Err(exchange::ExchangeError::build_box(format!(
                "wrong ordersheet type!"
            )))
        }
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

    fn withdrawl(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
    ) {
        let url = format!("{}/withdraw", exchange.api_url.as_str());
        let pub_addr = format!("0x{}", eth::privkey_to_addr(private_key));
        let nonce = self.geth.nonce(&pub_addr).unwrap();
        let base_token = &self.tokens.get(&token.symbol);
        let bigint = exchange::quantity_in_base_units(amount, 18, 18);
        let withdraw = WithdrawRequest {
            address: pub_addr,
            amount: bigint.to_str_radix(10),
            token: base_token.address.clone(),
            nonce: nonce.to_string(),
        };
        println!("{:?}", withdraw);
        let params_hash_bytes = withdraw_params_hash(&withdraw, &exchange.contract_address);
        let params_hash = eth::ethsign_hash_msg(&params_hash_bytes[..].to_vec());
        let private_key_bytes = &hex::decode(private_key).unwrap();
        let secret_key = SecretKey::from_slice(private_key_bytes).unwrap();
        let (v, r, s) = eth::sign_bytes_vrs(&params_hash, &secret_key);
        let signed = WithdrawRequestSigned {
            withdraw_request: withdraw,
            v: v,
            r: eth::hex(&r),
            s: eth::hex(&s),
        };
        let resp = self.client.post(url.as_str()).json(&signed).send().unwrap();
        let status = resp.status();
        let json = resp.text().unwrap();
        println!("{} {} {:?}", url, status, json);
        //{"error":"Invalid withdrawal signature. Please try again."}
    }

    fn deposit(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
    ) {
        println!("depositing {} amount {}", token.symbol, amount);
        let (data, value) = if token.symbol == "ETH" {
            let bigint = exchange::quantity_in_base_units(amount, 18, 18);
            (
                deposit_data(),
                ethereum_types::U256::from_dec_str(&bigint.to_str_radix(10)).unwrap(),
            )
        } else {
            let base_token = &self.tokens.get(&token.symbol);
            (
                deposit_token_data(&base_token.address, "0"),
                ethereum_types::U256::zero(),
            )
        };

        let pub_addr = format!("0x{}", eth::privkey_to_addr(private_key));
        let nonce = self.geth.nonce(&pub_addr).unwrap();
        let gas_price_fast = geth::ethgasstation_fast();
        let gas_price_gwei = gas_price_fast / 1_000_000_000u64;
        println!("TX Count/next nonce {} gas {}gwei", nonce, gas_price_gwei);

        let mut contract_addra = [0u8; 20];
        let contract_addr = exchange.contract_address.clone();
        contract_addra.copy_from_slice(&eth::dehex(&contract_addr)[..]);
        let tx = ethereum_tx_sign::RawTransaction {
            nonce: ethereum_types::U256::from(nonce),
            to: Some(ethereum_types::H160::from(contract_addra)),
            value: value,
            gas_price: ethereum_types::U256::from(gas_price_fast),
            gas: ethereum_types::U256::from(310240),
            data: data,
        };
        let private_key = ethereum_types::H256::from_slice(&eth::dehex(private_key));
        let rlp_bytes = tx.sign(&private_key, &eth::ETH_CHAIN_MAINNET);
        let params = (eth::hex(&rlp_bytes),);
        let result = self
            .geth
            .rpc("eth_sendRawTransaction", geth::ParamTypes::Single(params))
            .unwrap();
        match result.part {
            geth::ResultTypes::Error(e) => Err(exchange::ExchangeError::build_box(e.error.message)),
            geth::ResultTypes::Result(r) => {
                let tx = r.result;
                println!("GOOD TX {}", tx);
                Ok(tx)
            }
        };
    }
}

pub fn deposit_token_data(token_address: &str, amount: &str) -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("depositToken(address,uint256)").to_vec();
    call.append(&mut func);
    let mut p1 = hex::decode(eth::encode_addr2(token_address)).unwrap();
    call.append(&mut p1);
    let mut p2 = hex::decode(eth::encode_uint256(amount)).unwrap();
    call.append(&mut p2);
    call
}

pub fn deposit_data() -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("deposit()").to_vec();
    call.append(&mut func);
    call
}

pub fn withdraw_params_hash(wd: &WithdrawRequest, contract_address: &str) -> [u8; 32] {
    let parts: Vec<Vec<u8>> = vec![
        eth::encode_addr(contract_address),
        eth::encode_addr(&wd.token),
        eth::encode_uint256(&wd.amount),
        eth::encode_addr(&wd.address),
        eth::encode_uint256(&wd.nonce),
    ];
    parts_hash(parts)
}

pub fn order_params_hash(order: &OrderSheet, contract_address: &str) -> [u8; 32] {
    let expires = order.expires.to_string();
    let parts: Vec<Vec<u8>> = vec![
        eth::encode_addr(contract_address),
        eth::encode_addr(&order.token_buy),
        eth::encode_uint256(&order.amount_buy),
        eth::encode_addr(&order.token_sell),
        eth::encode_uint256(&order.amount_sell),
        eth::encode_uint256(&expires),
        eth::encode_uint256(&order.nonce),
        eth::encode_addr(&order.address),
    ];
    parts_hash(parts)
}

pub fn parts_hash(mut parts: Vec<Vec<u8>>) -> [u8; 32] {
    let hash_hex = parts.iter_mut().fold(Vec::<u8>::new(), |mut memo, part| {
        memo.append(part);
        memo
    });
    println!("parts {}", std::str::from_utf8(&hash_hex).unwrap());
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
